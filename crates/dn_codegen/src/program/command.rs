//! Commands: each works on a copy of its aggregate and gives back the changed
//! copy only if every precondition and invariant holds.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::{AggregateDecl, FunctionDecl};

use super::constraints::CheckPlace;
use crate::function::{OwnFields, Returns, Writer};
use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    pub fn commands(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();
        for found in self.model.all(SymbolKind::Aggregate) {
            let context = &self.program.contexts[&found.context];
            let Some(aggregate) = context
                .aggregates
                .iter()
                .find(|a| a.name.name == found.name)
            else {
                continue;
            };
            for command in &aggregate.commands {
                out.push_str(&self.command(&found, aggregate, command, prototypes));
            }
        }
        out
    }

    fn command(
        &self,
        found: &Resolved,
        aggregate: &AggregateDecl,
        command: &FunctionDecl,
        prototypes: &mut Vec<String>,
    ) -> String {
        let aggregate_type = Type::Named(found.clone());
        let result = Type::result(aggregate_type.clone(), Type::Errors(Vec::new()));
        let parameters = self.command_parameters(found, &command.name.name);
        let leading = [format!("const {} *p_self", self.c_type(&aggregate_type))];
        let name = format!("{}_{}", c_name(found), command.name.name);
        let signature = self.signature(&name, &leading, &parameters, Some(&result));
        prototypes.push(format!("{signature};"));
        let mut writer = Writer::new(self, &found.context, Returns::Command);
        writer.aggregate = Some(found.name.clone());
        let fields = self
            .model
            .fields(found)
            .into_iter()
            .map(|(name, ty, _)| (name, ty))
            .collect();
        writer.own = Some(OwnFields {
            prefix: "self.".to_string(),
            fields,
            owner: Some(found.clone()),
        });
        writer.reserve("p_self");
        let own_copy = writer.raw_variable("self", &self.c_type(&aggregate_type));
        let mark = writer.raw_variable("mark", "dn_u16");
        writer.line(format!("{own_copy} = *p_self;"));
        writer.line(format!("{mark} = dn_events_mark();"));
        writer.line(format!("(void) {mark};"));
        writer.bind_parameters(&parameters);
        let label = format!("{}.{}", found.name, command.name.name);
        let place = CheckPlace {
            context: &found.context,
            aggregate: Some(&found.name),
            label: &label,
        };
        let constraints = command
            .parameters
            .iter()
            .flatten()
            .map(|p| p.constraints.as_slice());
        let checked: Vec<(String, Type, &[dn_syntax::ast::ConstraintUse])> = parameters
            .iter()
            .zip(constraints)
            .map(|((name, ty), constraints)| (name.clone(), ty.clone(), constraints))
            .collect();
        if let Some(call) = self.argument_check(&name, &place, &checked) {
            writer.debug_call(&call);
        }
        writer.preconditions(&command.preconditions, &command.name.name);
        if let Some(body) = &command.body {
            writer.value(body, &Type::Unit);
        }
        for invariant in &aggregate.invariants {
            let constraint = self.model.resolve(
                &found.context,
                Some(&found.name),
                &invariant.constraint.name,
            );
            let Some(constraint) = constraint.filter(|c| c.kind == SymbolKind::Constraint) else {
                continue;
            };
            let rest: Vec<(String, Type)> = self
                .parameters_of(&constraint)
                .into_iter()
                .skip(1)
                .collect();
            let arguments: Vec<dn_syntax::ast::Argument> = (invariant.constraint.arguments.iter())
                .map(|value| dn_syntax::ast::Argument {
                    name: None,
                    value: value.clone(),
                })
                .collect();
            let mut passed = vec![format!("&{own_copy}")];
            passed.extend(writer.pass_all(&arguments, &rest));
            let holds = writer.call_into(&c_name(&constraint), passed, &Type::Bool);
            writer.open(format!("if (!{holds})"));
            let error = invariant
                .error
                .as_ref()
                .map_or("0".to_string(), |error| writer.invariant_error(error));
            writer.fail(&error);
            writer.close();
        }
        writer.check_fields(&own_copy, &aggregate_type);
        writer.line("out->ok = 1;");
        writer.line(format!("out->value = {own_copy};"));
        writer.finish(&signature)
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// The error after an invariant's `or`. Invariants are not bodies, so the
    /// checker left no notes on it; the name is looked up here.
    fn invariant_error(&mut self, error: &dn_syntax::ast::Expr) -> String {
        let path = crate::function::path_of(error);
        let aggregate = self.aggregate.clone();
        let found = path.and_then(|path| {
            self.gen
                .model
                .resolve(&self.context, aggregate.as_deref(), &path)
        });
        match found {
            Some(error) if error.kind == SymbolKind::Error => self.error_value(&error),
            _ => self.value(error, &Type::Errors(Vec::new())),
        }
    }
}
