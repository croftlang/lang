//! Debug-build checks of `where` constraints: on a function's arguments when it
//! starts, on its result before it returns, on a value's fields when it is
//! built, and of a port's contract when an adapter's operation starts. Each
//! check is its own function, compiled only with `CROFT_CHECKS`.

use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::{ConstraintUse, Expr};

use crate::function::{Returns, Writer};
use crate::names::c_name;
use crate::Generator;

/// Where a check's constraints are looked up, and how its messages name things.
pub struct CheckPlace<'r> {
    pub context: &'r str,
    pub aggregate: Option<&'r str>,
    pub label: &'r str,
}

impl<'a> Generator<'a> {
    /// A function that checks each argument against its constraints, and the call
    /// to it, when any argument has constraints.
    pub fn argument_check(
        &self,
        function: &str,
        place: &CheckPlace,
        parameters: &[(String, Type, &[ConstraintUse])],
    ) -> Option<String> {
        // A constraint with `or` fails the call instead; `constraint_failures` writes it.
        let must_hold =
            |constraints: &[ConstraintUse]| constraints.iter().any(|c| c.error.is_none());

        if !parameters
            .iter()
            .any(|(_, _, constraints)| must_hold(constraints))
        {
            return None;
        }

        let name = format!("croft_check_{function}");

        let typed: Vec<(String, Type)> = parameters
            .iter()
            .map(|(n, t, _)| (n.clone(), t.clone()))
            .collect();

        let mut writer = self.check_writer(place);
        writer.bind_parameters(&typed);

        for (parameter, ty, constraints) in parameters {
            let (value, _) = writer.lookup(parameter).expect("a bound parameter");

            for constraint in constraints.iter().filter(|c| c.error.is_none()) {
                writer.check_constraint(
                    &value,
                    ty,
                    constraint,
                    &format!("{}: {parameter}", place.label),
                );
            }
        }

        self.add_check(writer, &self.signature(&name, &[], &typed, None));

        let arguments: Vec<String> = typed.iter().map(|(n, _)| format!("p_{n}")).collect();

        Some(format!("{name}({});", arguments.join(", ")))
    }

    /// A function that checks the lines of a port operation's contract, such
    /// as `low <= high`, and the call to it with `arguments`, when it has any.
    /// The lines read the port's names for the parameters, whatever the
    /// adapter calls them.
    pub fn contract_check(
        &self,
        function: &str,
        place: &CheckPlace,
        parameters: &[(String, Type)],
        preconditions: &[Expr],
        arguments: &[String],
    ) -> Option<String> {
        if preconditions.is_empty() {
            return None;
        }

        let name = format!("croft_check_contract_{function}");

        let mut writer = self.check_writer(place);
        writer.bind_parameters(parameters);
        writer.preconditions(preconditions, place.label);

        self.add_check(writer, &self.signature(&name, &[], parameters, None));

        Some(format!("{name}({});", arguments.join(", ")))
    }

    /// A function that checks a result against its constraints, when it has any.
    pub fn result_check(
        &self,
        function: &str,
        place: &CheckPlace,
        ty: &Type,
        constraints: &[ConstraintUse],
    ) -> Option<String> {
        if constraints.is_empty() || matches!(ty, Type::Result(..)) {
            return None;
        }

        let name = format!("croft_check_result_{function}");
        let mut writer = self.check_writer(place);
        let parameters = [("result".to_string(), ty.clone())];
        writer.bind_parameters(&parameters);

        let (value, _) = writer.lookup("result").expect("a bound parameter");

        for constraint in constraints {
            writer.check_constraint(
                &value,
                ty,
                constraint,
                &format!("{}: the result", place.label),
            );
        }

        self.add_check(writer, &self.signature(&name, &[], &parameters, None));

        Some(name)
    }

    /// The function that checks a value's fields against their constraints, when
    /// any field has constraints. Generated once for each type.
    pub fn field_check(&self, named: &Resolved) -> Option<String> {
        let fields = self.model.fields(named);

        if fields
            .iter()
            .all(|(_, _, field)| field.constraints.is_empty())
        {
            return None;
        }

        let name = format!("croft_check_{}", c_name(named));

        if !self.checked_types.borrow_mut().insert(named.clone()) {
            return Some(name);
        }

        let aggregate = named.name.split('.').next().filter(|_| {
            named.name.contains('.') || named.kind == croft_analysis::symbols::SymbolKind::Aggregate
        });

        let place = CheckPlace {
            context: &named.context,
            aggregate,
            label: &named.name,
        };

        let mut writer = self.check_writer(&place);
        let ty = Type::Named(named.clone());
        writer.bind_parameters(&[("value".to_string(), ty.clone())]);

        for (field, field_type, decl) in &fields {
            let mut place = format!("p_value->{field}");

            if let Some((byte, mask)) = self.flag_bit(named, field) {
                place = format!("((p_value->croft_flags[{byte}] & {mask}) != 0)");
            }

            for constraint in &decl.constraints {
                writer.check_constraint(
                    &place,
                    field_type,
                    constraint,
                    &format!("{}: {field}", named.name),
                );
            }
        }

        self.add_check(
            writer,
            &self.signature(&name, &[], &[("value".to_string(), ty)], None),
        );

        Some(name)
    }

    /// A writer for the body of a check function at `place`.
    fn check_writer(&self, place: &CheckPlace) -> Writer<'_, 'a> {
        let mut writer = Writer::new(self, place.context, Returns::Nothing);
        writer.aggregate = place.aggregate.map(str::to_string);
        writer
    }

    /// Keeps a finished check function and its prototype.
    fn add_check(&self, writer: Writer, signature: &str) {
        self.checks
            .borrow_mut()
            .push((format!("{signature};"), writer.finish(signature)));
    }

    /// Every check function, only when checks are on.
    pub fn check_functions(&self) -> (String, String) {
        let checks = self.checks.borrow();

        if checks.is_empty() {
            return (String::new(), String::new());
        }

        let prototypes: Vec<&str> = checks.iter().map(|(p, _)| p.as_str()).collect();
        let definitions: String = checks.iter().map(|(_, d)| d.as_str()).collect();

        (
            format!("#ifdef CROFT_CHECKS\n{}\n#endif\n\n", prototypes.join("\n")),
            format!("#ifdef CROFT_CHECKS\n{definitions}#endif\n\n"),
        )
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// A call that only happens when checks are on.
    pub fn checked_call(&mut self, call: &str) {
        self.directive("#ifdef CROFT_CHECKS");
        self.line(call);
        self.directive("#endif");
    }

    /// Checks a value just built against its type's field constraints, when checks are on.
    pub fn check_fields(&mut self, value: &str, ty: &Type) {
        let Type::Named(named) = ty else { return };

        if let Some(check) = self.gen.field_check(named) {
            self.checked_call(&format!("{check}(&{value});"));
        }
    }
}
