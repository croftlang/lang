//! Commands and factories. A command works on a copy of its aggregate and
//! gives back the changed copy only if every constraint and invariant holds;
//! a factory makes a new one, held to the same invariants.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::{AggregateDecl, FunctionDecl, Invariant};

use super::constraints::CheckPlace;
use crate::function::{OwnFields, Returns, Writer};
use crate::names::{c_name, event_kind};
use crate::Generator;

impl<'a> Generator<'a> {
    /// Every factory and command of every aggregate, as C. Adds their
    /// prototypes to `prototypes`.
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

            for factory in &aggregate.factories {
                out.push_str(&self.factory(&found, aggregate, factory, prototypes));
            }

            for command in &aggregate.commands {
                out.push_str(&self.command(&found, aggregate, command, prototypes));
            }
        }

        out
    }

    /// A command as a C function taking its aggregate as `p_self`: runs the
    /// body on a copy, and gives back the changed copy only if every
    /// constraint and invariant holds.
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

        let declared: Vec<&[croft_syntax::ast::ConstraintUse]> = (command.parameters.iter())
            .flatten()
            .map(|p| p.constraints.as_slice())
            .collect();

        let ranges = self.parameter_ranges(&found.context, &parameters, &declared);
        let signature = self.ranged_signature(&name, &leading, &parameters, &ranges, Some(&result));
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
        let mark = writer.raw_variable("mark", "croft_u16");

        writer.line(format!("{own_copy} = *p_self;"));
        writer.line(format!("{mark} = croft_events_mark();"));
        writer.line(format!("(void) {mark};"));
        writer.bind_parameters(&parameters);
        writer.note_parameter_ranges(&parameters, &ranges);

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

        let checked: Vec<(String, Type, &[croft_syntax::ast::ConstraintUse])> = parameters
            .iter()
            .zip(constraints)
            .map(|((name, ty), constraints)| (name.clone(), ty.clone(), constraints))
            .collect();

        if let Some(call) = self.argument_check(&name, &place, &checked) {
            writer.checked_call(&call);
        }

        writer.constraint_failures(&checked);
        writer.preconditions(&command.preconditions, &command.name.name);

        if let Some(body) = &command.body {
            writer.value(body, &Type::Unit);
        }

        writer.invariants(found, aggregate, &own_copy);
        writer.check_fields(&own_copy, &aggregate_type);
        writer.line("out->ok = 1;");
        writer.line(format!("out->value = {own_copy};"));

        writer.finish(&signature)
    }

    /// `Cargo_Book(args..., out)`: runs the body, which makes the aggregate,
    /// and gives it back only if every constraint and invariant holds.
    fn factory(
        &self,
        found: &Resolved,
        aggregate: &AggregateDecl,
        factory: &FunctionDecl,
        prototypes: &mut Vec<String>,
    ) -> String {
        let aggregate_type = Type::Named(found.clone());
        let result = Type::result(aggregate_type.clone(), Type::Errors(Vec::new()));
        let parameters = self.factory_parameters(found, &factory.name.name);
        let name = format!("{}_{}", c_name(found), factory.name.name);
        let signature = self.signature(&name, &[], &parameters, Some(&result));
        prototypes.push(format!("{signature};"));

        let mut writer = Writer::new(self, &found.context, Returns::Command);
        writer.aggregate = Some(found.name.clone());
        writer.factory = true;

        let made = writer.raw_variable("self", &self.c_type(&aggregate_type));
        let mark = writer.raw_variable("mark", "croft_u16");

        writer.line(format!("{mark} = croft_events_mark();"));
        writer.line(format!("(void) {mark};"));
        writer.bind_parameters(&parameters);

        let label = format!("{}.{}", found.name, factory.name.name);

        let place = CheckPlace {
            context: &found.context,
            aggregate: Some(&found.name),
            label: &label,
        };

        let constraints = factory
            .parameters
            .iter()
            .flatten()
            .map(|p| p.constraints.as_slice());

        let checked: Vec<(String, Type, &[croft_syntax::ast::ConstraintUse])> = parameters
            .iter()
            .zip(constraints)
            .map(|((name, ty), constraints)| (name.clone(), ty.clone(), constraints))
            .collect();

        if let Some(call) = self.argument_check(&name, &place, &checked) {
            writer.checked_call(&call);
        }

        writer.constraint_failures(&checked);
        writer.preconditions(&factory.preconditions, &factory.name.name);

        if let Some(body) = &factory.body {
            let value = writer.value(body, &aggregate_type);
            writer.line(format!("{made} = {value};"));
        }

        writer.invariants(found, aggregate, &made);
        writer.check_fields(&made, &aggregate_type);
        writer.claim_events(found, &mark, &made);
        writer.line("out->ok = 1;");
        writer.line(format!("out->value = {made};"));

        writer.finish(&signature)
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// Fails with an invariant's error when `value` breaks it.
    fn invariants(&mut self, found: &Resolved, aggregate: &AggregateDecl, value: &str) {
        for invariant in &aggregate.invariants {
            let Some(holds) = self.invariant_holds(found, invariant, value) else {
                continue;
            };

            self.open(format!("if (!{holds})"));

            let error = invariant
                .error
                .as_ref()
                .map_or("0".to_string(), |error| self.invariant_error(error));

            self.fail(&error);
            self.close();
        }
    }

    /// A C expression that is true when the aggregate `value` keeps
    /// `invariant`, or `None` when it names no constraint.
    pub(crate) fn invariant_holds(
        &mut self,
        found: &Resolved,
        invariant: &Invariant,
        value: &str,
    ) -> Option<String> {
        let constraint = self.gen.model.resolve(
            &found.context,
            Some(&found.name),
            &invariant.constraint.name,
        );

        let constraint = constraint.filter(|c| c.kind == SymbolKind::Constraint)?;

        let rest: Vec<(String, Type)> = self
            .gen
            .parameters_of(&constraint)
            .into_iter()
            .skip(1)
            .collect();

        let arguments: Vec<croft_syntax::ast::Argument> = (invariant.constraint.arguments.iter())
            .map(|value| croft_syntax::ast::Argument {
                name: None,
                value: value.clone(),
            })
            .collect();

        let mut passed = vec![format!("&{value}")];
        passed.extend(self.pass_all(&arguments, &rest));

        Some(self.call_into(&c_name(&constraint), passed, &Type::Bool))
    }

    /// Gives each event a factory raised its source and key, which only
    /// exist once the body has made the aggregate.
    fn claim_events(&mut self, owner: &Resolved, mark: &str, made: &str) {
        let emitted = std::mem::take(&mut self.emitted);

        if emitted.is_empty() {
            return;
        }

        let identity = format!("{made}.{}", self.gen.identity_field(owner));
        let key = self.gen.event_key(owner, &identity);
        let index = self.raw_variable("index", "croft_u16");

        self.open(format!(
            "for ({index} = {mark}; {index} < croft_events_mark(); {index}++)"
        ));

        self.line(format!("croft_events_at({index})->key = {key};"));

        for event in emitted {
            self.line(format!(
                "if (croft_events_at({index})->kind == {}) croft_events_at({index})->as.{}.source = {identity};",
                event_kind(&event),
                c_name(&event)
            ));
        }

        self.close();
    }

    /// The error after an invariant's `or`. Invariants are not bodies, so the
    /// checker left no notes on it; the name is looked up here.
    fn invariant_error(&mut self, error: &croft_syntax::ast::Expr) -> String {
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
