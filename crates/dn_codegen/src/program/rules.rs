//! Debug-build checks of `where` rules: on a function's arguments when it
//! starts, on its result before it returns, and on a value's fields when it
//! is built. Each check is its own function, compiled only with `DN_DEBUG`.

use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::Rule;

use crate::function::{Returns, Writer};
use crate::names::c_name;
use crate::Generator;

/// Where a check's rules are looked up, and how its messages name things.
pub struct CheckPlace<'r> {
    pub context: &'r str,
    pub aggregate: Option<&'r str>,
    pub label: &'r str,
}

impl<'a> Generator<'a> {
    /// A function that checks each argument against its rules, and the call
    /// to it, when any argument has rules.
    pub fn argument_check(
        &self,
        function: &str,
        place: &CheckPlace,
        parameters: &[(String, Type, &[Rule])],
    ) -> Option<String> {
        if parameters.iter().all(|(_, _, rules)| rules.is_empty()) {
            return None;
        }
        let name = format!("dn_check_{function}");
        let typed: Vec<(String, Type)> = parameters
            .iter()
            .map(|(n, t, _)| (n.clone(), t.clone()))
            .collect();
        let mut writer = self.check_writer(place);
        writer.bind_parameters(&typed);
        for (parameter, ty, rules) in parameters {
            let (value, _) = writer.lookup(parameter).expect("a bound parameter");
            for rule in rules.iter() {
                writer.check_rule(&value, ty, rule, &format!("{}: {parameter}", place.label));
            }
        }
        self.add_check(writer, &self.signature(&name, &[], &typed, None));
        let arguments: Vec<String> = typed.iter().map(|(n, _)| format!("p_{n}")).collect();
        Some(format!("{name}({});", arguments.join(", ")))
    }

    /// A function that checks a result against its rules, when it has any.
    pub fn result_check(
        &self,
        function: &str,
        place: &CheckPlace,
        ty: &Type,
        rules: &[Rule],
    ) -> Option<String> {
        if rules.is_empty() || matches!(ty, Type::Result(..)) {
            return None;
        }
        let name = format!("dn_check_result_{function}");
        let mut writer = self.check_writer(place);
        let parameters = [("result".to_string(), ty.clone())];
        writer.bind_parameters(&parameters);
        let (value, _) = writer.lookup("result").expect("a bound parameter");
        for rule in rules {
            writer.check_rule(&value, ty, rule, &format!("{}: the result", place.label));
        }
        self.add_check(writer, &self.signature(&name, &[], &parameters, None));
        Some(name)
    }

    /// The function that checks a value's fields against their rules, when
    /// any field has rules. Generated once for each type.
    pub fn field_check(&self, named: &Resolved) -> Option<String> {
        let fields = self.model.fields(named);
        if fields.iter().all(|(_, _, field)| field.rules.is_empty()) {
            return None;
        }
        let name = format!("dn_check_{}", c_name(named));
        if !self.checked_types.borrow_mut().insert(named.clone()) {
            return Some(name);
        }
        let aggregate = named.name.split('.').next().filter(|_| {
            named.name.contains('.') || named.kind == dn_analysis::symbols::SymbolKind::Aggregate
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
            for rule in &decl.rules {
                writer.check_rule(
                    &format!("p_value->{field}"),
                    field_type,
                    rule,
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

    fn check_writer(&self, place: &CheckPlace) -> Writer<'_, 'a> {
        let mut writer = Writer::new(self, place.context, Returns::Nothing);
        writer.aggregate = place.aggregate.map(str::to_string);
        writer
    }

    fn add_check(&self, writer: Writer, signature: &str) {
        self.checks
            .borrow_mut()
            .push((format!("{signature};"), writer.finish(signature)));
    }

    /// Every check function, for debug builds only.
    pub fn check_functions(&self) -> (String, String) {
        let checks = self.checks.borrow();
        if checks.is_empty() {
            return (String::new(), String::new());
        }
        let prototypes: Vec<&str> = checks.iter().map(|(p, _)| p.as_str()).collect();
        let definitions: String = checks.iter().map(|(_, d)| d.as_str()).collect();
        (
            format!("#ifdef DN_DEBUG\n{}\n#endif\n\n", prototypes.join("\n")),
            format!("#ifdef DN_DEBUG\n{definitions}#endif\n\n"),
        )
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// A call that only happens in debug builds.
    pub fn debug_call(&mut self, call: &str) {
        self.directive("#ifdef DN_DEBUG");
        self.line(call);
        self.directive("#endif");
    }

    /// Checks a value just built against its type's field rules, in debug builds.
    pub fn check_fields(&mut self, value: &str, ty: &Type) {
        let Type::Named(named) = ty else { return };
        if let Some(check) = self.gen.field_check(named) {
            self.debug_call(&format!("{check}(&{value});"));
        }
    }
}
