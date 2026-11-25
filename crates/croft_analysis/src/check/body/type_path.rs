//! Names that stand for declarations: constants, errors and enum variants.

use croft_syntax::ast::{ExprId, Path};

use super::BodyChecker;
use crate::facts::Meaning;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A name that stands for a constant, an error or an enum variant.
    pub(super) fn type_path(
        &mut self,
        id: Option<ExprId>,
        path: &Path,
        expected: Option<&Type>,
    ) -> Option<Type> {
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

                let ty = self.checker.lower_type(&scope, &constant.ty);

                self.note(id, Meaning::Const(found));
                return Some(ty);
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

                self.note(id, Meaning::Error(found.clone()));
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

        if let Type::Named(owner) = &ty {
            self.note(
                id,
                Meaning::Variant(owner.clone(), variant.name.clone(), variant.error.clone()),
            );
        }

        Some(ty)
    }

    /// Records what an expression refers to, when it is one.
    pub(super) fn note(&mut self, id: Option<ExprId>, meaning: Meaning) {
        if let Some(id) = id {
            self.checker.record(id, meaning);
        }
    }
}
