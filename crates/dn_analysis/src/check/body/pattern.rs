//! `match`: every variant handled once, with bindings for its fields.

use dn_syntax::ast::{Expr, MatchArm, Pattern};

use super::BodyChecker;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn check_match(
        &mut self,
        subject: &Expr,
        arms: &[MatchArm],
        expected: Option<&Type>,
    ) -> Type {
        let subject_type = self.expr(subject, None);
        let Some(variants) = self.match_variants(&subject_type) else {
            self.error(format!("cannot match on `{subject_type}`"), subject.span);
            return Type::Unknown;
        };
        let mut result = expected.cloned();
        let mut handled: Vec<String> = Vec::new();
        for arm in arms {
            let name = arm.pattern.path.last().name.clone();
            let bindings = match variants.iter().find(|(variant, _)| *variant == name) {
                Some((_, fields)) => self.pattern_bindings(&arm.pattern, fields),
                None => {
                    self.error(
                        format!("`{subject_type}` has no variant `{name}`"),
                        arm.pattern.path.span(),
                    );
                    vec![Type::Unknown; arm.pattern.bindings.len()]
                }
            };
            if handled.contains(&name) {
                self.error(
                    format!("`{name}` is handled more than once"),
                    arm.pattern.path.span(),
                );
            }
            handled.push(name);
            let found = self.nested(|body| {
                for (binding, ty) in arm.pattern.bindings.iter().zip(bindings) {
                    body.bind(&binding.name, ty, false);
                }
                match &result {
                    Some(expected) => body.expect(&arm.value, expected),
                    None => body.expr(&arm.value, None),
                }
            });
            if result.as_ref().is_none_or(Type::is_unknown) {
                result = Some(found);
            }
        }
        let missing: Vec<&str> = (variants.iter())
            .map(|(name, _)| name.as_str())
            .filter(|name| !handled.iter().any(|h| h == name))
            .collect();
        if !missing.is_empty() && !subject_type.is_unknown() {
            self.error(
                format!("this match does not handle {}", quote_all(&missing)),
                subject.span,
            );
        }
        result.unwrap_or(Type::Unknown)
    }

    /// The variants of the type being matched, each with its fields' types.
    pub(super) fn match_variants(&self, ty: &Type) -> Option<Vec<(String, Vec<Type>)>> {
        match ty {
            Type::Unknown => Some(Vec::new()),
            Type::Option(item) => Some(vec![
                ("Some".into(), vec![(**item).clone()]),
                ("None".into(), Vec::new()),
            ]),
            Type::Result(value, error) => Some(vec![
                ("Ok".into(), vec![(**value).clone()]),
                ("Err".into(), vec![(**error).clone()]),
            ]),
            Type::Named(named)
                if matches!(
                    named.kind,
                    SymbolKind::Enum | SymbolKind::ErrorSet | SymbolKind::Error
                ) =>
            {
                let variants = self.checker.variants_of(named).into_iter().map(|variant| {
                    let fields = variant
                        .fields
                        .iter()
                        .map(|f| self.checker.lower_type(&variant.scope, &f.ty))
                        .collect();
                    (variant.name, fields)
                });
                Some(variants.collect())
            }
            _ => None,
        }
    }

    /// A pattern binds every field of its variant, or none of them.
    pub(super) fn pattern_bindings(&mut self, pattern: &Pattern, fields: &[Type]) -> Vec<Type> {
        let count = pattern.bindings.len();
        if count != 0 && count != fields.len() {
            let name = pattern.path.text();
            let message = format!(
                "`{name}` has {} fields, but the pattern names {count}",
                fields.len()
            );
            self.error(message, pattern.path.span());
        }
        let mut types = fields.to_vec();
        types.resize(count, Type::Unknown);
        types
    }
}

fn quote_all(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    quoted.join(", ")
}
