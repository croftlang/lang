//! `match`: every variant handled once, with bindings for its fields, or
//! the rest handled together by a last arm of `_`.

use croft_syntax::ast::{Expr, MatchArm, Pattern};

use super::BodyChecker;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a `match`: its arms give back the same type, and every
    /// variant is handled once.
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
        let mut has_wildcard = false;

        for (index, arm) in arms.iter().enumerate() {
            let name = arm.pattern.path.last().name.clone();

            self.note_pattern(&subject_type, &arm.pattern);

            if name == WILDCARD {
                self.check_wildcard(arm, index + 1 == arms.len(), &variants, &handled);
                has_wildcard = true;
            }

            let bindings = match variants.iter().find(|(variant, _)| *variant == name) {
                _ if name == WILDCARD => Vec::new(),
                Some((_, fields)) => self.pattern_bindings(&arm.pattern, fields),
                None => {
                    self.error(
                        format!("`{subject_type}` has no variant `{name}`"),
                        arm.pattern.path.span(),
                    );

                    vec![Type::Unknown; arm.pattern.bindings.len()]
                }
            };

            if handled.contains(&name) && !has_wildcard {
                self.error(
                    format!("`{name}` is handled more than once"),
                    arm.pattern.path.span(),
                );
            }

            handled.push(name);

            let found = self.nested(|body| {
                for (binding, ty) in arm.pattern.bindings.iter().zip(bindings) {
                    body.bind(&binding.name, ty, false, Some(binding.span));
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

        if !missing.is_empty() && !subject_type.is_unknown() && !has_wildcard {
            self.error(
                format!("this match does not handle {}", quote_all(&missing)),
                subject.span,
            );
        }

        result.unwrap_or(Type::Unknown)
    }

    /// `_` handles every variant no arm above it does, so it comes last,
    /// binds nothing, and has at least one variant left to handle.
    fn check_wildcard(
        &mut self,
        arm: &MatchArm,
        is_last: bool,
        variants: &[(String, Vec<Type>)],
        handled: &[String],
    ) {
        let span = arm.pattern.path.span();

        if !is_last {
            self.error(
                "`_` handles everything left, so it must be the last arm",
                span,
            );
        }

        if !arm.pattern.bindings.is_empty() {
            self.error(
                "`_` stands for several variants, so it can't name their fields",
                span,
            );
        }

        let left = variants
            .iter()
            .any(|(variant, _)| !handled.contains(variant));

        if !left && !variants.is_empty() {
            self.error(
                "every variant is already handled, so `_` is never reached",
                span,
            );
        }
    }

    /// Notes what a pattern's variant refers to, for editors: the error it
    /// stands for, or the enum that declares it.
    fn note_pattern(&mut self, subject: &Type, pattern: &Pattern) {
        let written = pattern.path.last();

        let built_in = match subject {
            Type::Option(_) => ["Some", "None"].contains(&written.name.as_str()),
            Type::Result(..) => ["Ok", "Err"].contains(&written.name.as_str()),
            _ => false,
        };

        if built_in {
            self.checker.facts.builtins.push(crate::facts::BuiltinName {
                span: written.span,
                name: written.name.clone(),
            });

            return;
        }

        let Type::Named(named) = subject else {
            return;
        };

        let variants = self.checker.variants_of(named);

        let Some(variant) = variants.into_iter().find(|v| v.name == written.name) else {
            return;
        };

        let reference = crate::facts::Reference {
            span: written.span,
            name: written.name.clone(),
            target: variant.error.unwrap_or(variant.declared_in),
        };

        self.checker.references.get_mut().push(reference);
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

/// The pattern that matches every variant not handled above it.
const WILDCARD: &str = "_";

/// Each name in backticks, joined with commas.
fn quote_all(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    quoted.join(", ")
}
