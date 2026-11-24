//! Names, fields, records and `with`.

use dn_syntax::ast::{Identifier, Path};
use dn_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn name(&mut self, name: &str, span: Span, expected: Option<&Type>) -> Type {
        if let Some(local) = self.local(name) {
            return local.ty.clone();
        }
        if let Some(ty) = self.own_field(name, span, false) {
            return ty;
        }
        match name {
            "Unit" => return Type::Unit,
            "None" => return Type::option(Type::Unknown),
            _ => {}
        }
        let path = Path {
            segments: vec![Identifier {
                name: name.to_string(),
                span,
            }],
        };
        if let Some(ty) = self.type_path(&path, expected) {
            return ty;
        }
        self.error(format!("cannot find `{name}` here"), span);
        Type::Unknown
    }

    /// A field of the command's aggregate or of the adapter. `for_set` leaves
    /// out the identity, which never changes.
    pub(super) fn own_field(&self, name: &str, _span: Span, for_set: bool) -> Option<Type> {
        let fields = match self.kind {
            BodyKind::Command(aggregate) => &aggregate.fields,
            BodyKind::Adapter(adapter) => &adapter.fields,
            _ => return None,
        };
        let field = fields
            .iter()
            .find(|field| field.name.name == name && !(for_set && field.is_identity))?;
        Some(self.checker.lower_type(&self.scope, &field.ty))
    }

    /// A name that stands for a constant, an error or an enum variant.
    pub(super) fn type_path(&mut self, path: &Path, expected: Option<&Type>) -> Option<Type> {
        let found = self.checker.resolve(&self.scope, path);
        match found.as_ref().map(|found| found.kind) {
            Some(SymbolKind::Const) => {
                let found = found.unwrap();
                let scope = self.checker.scope_of(&found);
                let constant = scope
                    .context
                    .consts
                    .iter()
                    .find(|c| c.name.name == found.name)?;
                return Some(self.checker.lower_type(&scope, &constant.ty));
            }
            Some(SymbolKind::Error) => {
                let found = found.unwrap();
                if self
                    .checker
                    .fields_of(&found)
                    .is_some_and(|fields| !fields.is_empty())
                {
                    let message = format!(
                        "`{}` needs its fields: `{} {{ ... }}`",
                        path.text(),
                        path.text()
                    );
                    self.error(message, path.span());
                }
                return Some(Type::Named(found));
            }
            Some(kind) => {
                let message = format!("`{}` is {}, not a value", path.text(), kind.describe());
                self.error(message, path.span());
                return Some(Type::Unknown);
            }
            None => {}
        }
        let (ty, variant) = self.find_variant(path, expected)?;
        if !variant.fields.is_empty() {
            let message = format!(
                "`{}` needs its fields: `{}(...)`",
                variant.name,
                path.text()
            );
            self.error(message, path.span());
        }
        Some(ty)
    }
}
