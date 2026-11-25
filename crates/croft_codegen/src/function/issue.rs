//! Issuing commands, and raising events from inside one.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Expr, ExprKind, FieldValue, Issue, Path};

use super::Writer;
use crate::names::{aggregate_kind, c_name, event_kind};

impl<'g, 'a> Writer<'g, 'a> {
    pub fn issue(&mut self, value: &Expr, issue: &Issue, ty: &Type) -> String {
        let meaning = self.gen.facts.meaning(value.id).cloned();

        let (aggregate, by_identity) = match meaning {
            Some(Meaning::IssueOnValue(aggregate)) => (aggregate, false),
            Some(Meaning::IssueByIdentity(aggregate)) => (aggregate, true),
            _ => return "0".to_string(),
        };

        let command = format!("{}_{}", c_name(&aggregate), issue.command.name);
        let parameters = self.gen.command_parameters(&aggregate, &issue.command.name);
        let aggregate_type = Type::Named(aggregate.clone());

        if !by_identity {
            let target = self.value(&issue.target, &aggregate_type);
            let mut passed = vec![format!("&{target}")];
            passed.extend(self.pass_all(&issue.arguments, &parameters));
            return self.call_into(&command, passed, ty);
        }

        let ExprKind::Call(_, identity) = &issue.target.kind else {
            return "0".to_string();
        };

        let identity_type = self.gen.model.identity_of(&aggregate);

        let identity = identity
            .first()
            .map_or("0".to_string(), |a| self.value(&a.value, &identity_type));

        let Some(repository) = self.gen.model.repository_for(&aggregate) else {
            return "0".to_string();
        };

        let Some((find, save)) = self.gen.model.find_and_save(&repository) else {
            return "0".to_string();
        };

        let repository = c_name(&repository);
        let (find, save) = (find.name, save.name);
        let changed_type = Type::result(aggregate_type, Type::Errors(Vec::new()));
        let found = self.temp(&changed_type);
        let changed = self.temp(&changed_type);
        let result = self.temp(ty);

        self.line(format!("{repository}_{find}({identity}, &{found});"));
        self.open(format!("if (!{found}.ok)"));
        self.line(format!("{result}.ok = 0;"));
        self.line(format!("{result}.error = {found}.error;"));
        self.close();
        self.open("else");

        let mut passed = vec![format!("&{found}.value")];
        passed.extend(self.pass_all(&issue.arguments, &parameters));
        passed.push(format!("&{changed}"));
        self.line(format!("{command}({});", passed.join(", ")));
        self.open(format!("if (!{changed}.ok)"));
        self.line(format!("{result}.ok = 0;"));
        self.line(format!("{result}.error = {changed}.error;"));
        self.close();

        self.line(format!(
            "else {repository}_{save}(&{changed}.value, &{result});"
        ));

        self.close();

        result
    }

    /// `emit EnemyDamaged { ... }`: queued until its aggregate is saved, and
    /// handled once the service ends.
    pub fn emit(&mut self, event: &Path, fields: &[FieldValue]) -> String {
        let Some(aggregate) = self.aggregate.clone() else {
            return "0".to_string();
        };

        let Some(found) = self
            .gen
            .model
            .lookup(&self.context, &format!("{aggregate}.{}", event.text()))
        else {
            return "0".to_string();
        };

        let name = c_name(&found);
        let queued = self.raw_variable("event", "croft_event");
        self.line(format!("{queued}.kind = {};", event_kind(&found)));

        let types = self.field_types(&found);

        for given in fields {
            let field = &given.name.name;

            let field_type = types
                .iter()
                .find(|(f, _)| f == field)
                .map_or(Type::Unknown, |(_, t)| t.clone());

            let value = match &given.value {
                Some(value) => self.value(value, &field_type),
                None => self.read_name(field),
            };

            self.write_field(Some(&found), &format!("{queued}.as.{name}"), field, &value);
        }

        let owner = Resolved {
            context: self.context.clone(),
            name: aggregate,
            kind: SymbolKind::Aggregate,
        };

        self.line(format!("{queued}.aggregate = {};", aggregate_kind(&owner)));

        // A factory's aggregate does not exist yet: its source and key are
        // filled in once the body has made it.
        if self.factory {
            self.line(format!("{queued}.key = 0;"));

            if !self.emitted.contains(&found) {
                self.emitted.push(found.clone());
            }
        } else {
            let identity = self.gen.identity_field(&owner);
            self.line(format!("{queued}.as.{name}.source = self.{identity};"));

            self.line(format!(
                "{queued}.key = {};",
                self.gen.event_key(&owner, &format!("self.{identity}"))
            ));
        }

        self.line(format!("croft_events_push(&{queued});"));

        "0".to_string()
    }
}
