//! `match`, written as a chain of `if`s so a `break` inside an arm still
//! ends the loop around it. A last arm of `_` is the final `else`.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, MatchArm, Pattern};

use super::Writer;
use crate::names::{c_name, error_code};

impl<'g, 'a> Writer<'g, 'a> {
    pub fn check_match(&mut self, subject: &Expr, arms: &[MatchArm], ty: &Type) -> String {
        let subject_type = self.type_of(subject, None);
        let (subject, _) = self.eval(subject, None);
        let result = (*ty != Type::Unit).then(|| self.temp(ty));

        for (index, arm) in arms.iter().enumerate() {
            let (test, bindings) = self.arm_test(&subject, &subject_type, &arm.pattern);

            if index == 0 {
                self.open(format!("if ({test})"));
            } else {
                self.close();

                let header = if index + 1 == arms.len() {
                    "else".to_string()
                } else {
                    format!("else if ({test})")
                };

                self.open(header);
            }

            self.push_scope();

            for (binding, (c, binding_type)) in arm.pattern.bindings.iter().zip(bindings) {
                self.bind(&binding.name, c, binding_type);
            }

            let value = self.value(&arm.value, ty);

            if let Some(result) = &result {
                self.line(format!("{result} = {value};"));
            }

            self.pop_scope();
        }

        if !arms.is_empty() {
            self.close();
        }

        result.unwrap_or_else(|| "0".to_string())
    }

    /// The test for one arm, and what each of its bindings refers to.
    fn arm_test(
        &mut self,
        subject: &str,
        ty: &Type,
        pattern: &Pattern,
    ) -> (String, Vec<(String, Type)>) {
        let name = pattern.path.last().name.as_str();

        if name == "_" {
            return ("1".to_string(), Vec::new());
        }

        match ty {
            Type::Option(item) if name == "Some" => (
                format!("{subject}.some"),
                vec![(format!("{subject}.value"), (**item).clone())],
            ),
            Type::Option(_) => (format!("!{subject}.some"), Vec::new()),
            Type::Result(value, _) if name == "Ok" => (
                format!("{subject}.ok"),
                vec![(format!("{subject}.value"), (**value).clone())],
            ),
            Type::Result(_, error) => (
                format!("!{subject}.ok"),
                vec![(format!("{subject}.error"), (**error).clone())],
            ),
            Type::Named(owner) => {
                let variants = self.gen.model.variants(owner);

                let Some(variant) = variants.into_iter().find(|v| v.name == name) else {
                    return ("0".to_string(), Vec::new());
                };

                if self.gen.model.is_error(ty) {
                    let error = self.gen.own_error(owner, &variant);
                    let prefix = format!("{subject}.fields.{}", c_name(&error));

                    let bindings = variant
                        .fields
                        .into_iter()
                        .map(|(f, t)| (format!("{prefix}.{f}"), t))
                        .collect();

                    return (
                        format!("{subject}.code == {}", error_code(&error)),
                        bindings,
                    );
                }

                let tag = format!("{}_{name}", c_name(owner));

                if self.gen.is_plain_enum(owner) {
                    return (format!("{subject} == {tag}"), Vec::new());
                }

                let bindings = variant
                    .fields
                    .into_iter()
                    .map(|(f, t)| (format!("{subject}.as.{name}.{f}"), t))
                    .collect();

                (format!("{subject}.tag == {tag}"), bindings)
            }
            _ => ("0".to_string(), Vec::new()),
        }
    }
}
