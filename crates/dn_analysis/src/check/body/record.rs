//! Records, `with`, and the field values they are given.

use dn_syntax::ast::{Expr, FieldValue, Path};

use super::BodyChecker;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks each field value against the field it names in `ty`.
    pub(super) fn field_values(&mut self, ty: &Type, fields: &[FieldValue]) {
        let mut seen = Vec::new();
        for given in fields {
            let name = &given.name;
            if seen.contains(&name.name) {
                self.error(
                    format!("`{}` is given more than once", name.name),
                    name.span,
                );
            }
            seen.push(name.name.clone());
            let field_type = match ty {
                Type::Named(named) => self.checker.field_type(named, &name.name),
                Type::Unknown => Some(Type::Unknown),
                _ => None,
            };
            let Some(field_type) = field_type else {
                self.error(format!("`{ty}` has no field `{}`", name.name), name.span);
                continue;
            };
            match &given.value {
                Some(value) => {
                    self.expect(value, &field_type);
                }
                None => {
                    let found = self.name(&name.name, name.span, Some(&field_type));
                    if !self.checker.accepts(&field_type, &found) {
                        let message = format!(
                            "expected `{field_type}` for `{}`, found `{found}`",
                            name.name
                        );
                        self.error(message, name.span);
                    }
                }
            }
        }
    }

    /// `Position { x: 10, y: 20 }`: every field, each once.
    pub(super) fn record(&mut self, path: &Path, fields: &[FieldValue]) -> Type {
        let kinds = [
            SymbolKind::Value,
            SymbolKind::Entity,
            SymbolKind::Error,
            SymbolKind::Aggregate,
        ];
        let Some(found) =
            self.checker
                .expect_kind(&self.scope, path, &kinds, "a value, entity or error")
        else {
            return Type::Unknown;
        };
        let declared = self.checker.fields_of(&found).unwrap_or(&[]);
        let names: Vec<String> = declared
            .iter()
            .map(|field| field.name.name.clone())
            .collect();
        for name in &names {
            if !fields.iter().any(|given| &given.name.name == name) {
                self.error(
                    format!("`{}` needs a value for `{name}`", path.text()),
                    path.span(),
                );
            }
        }
        let ty = Type::Named(found);
        self.field_values(&ty, fields);
        ty
    }

    /// `start with { x: 11 }`: a copy with some fields replaced.
    pub(super) fn with(&mut self, base: &Expr, fields: &[FieldValue]) -> Type {
        let ty = self.expr(base, None);
        self.field_values(&ty, fields);
        ty
    }
}
