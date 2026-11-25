//! What the model says about types: their variants and errors, and every declaration of a kind.

use super::model::{Model, VariantInfo};
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'a> Model<'a> {
    pub fn variants(&self, named: &Resolved) -> Vec<VariantInfo> {
        let variants = self.checker.variants_of(named);
        variants
            .into_iter()
            .map(|variant| VariantInfo {
                fields: (variant.fields.iter())
                    .map(|f| {
                        (
                            f.name.name.clone(),
                            self.checker.lower_type(&variant.scope, &f.ty),
                        )
                    })
                    .collect(),
                name: variant.name,
                error: variant.error,
            })
            .collect()
    }

    /// Whether values of this type are errors, which share one C type.
    pub fn is_error(&self, ty: &Type) -> bool {
        match ty {
            Type::Errors(_) => true,
            Type::Named(named) => self.checker.is_error_type(named),
            _ => false,
        }
    }

    pub fn error_members(&self, named: &Resolved) -> Vec<Resolved> {
        self.checker.error_members(named)
    }

    /// Every declaration of one kind, across all contexts.
    pub fn all(&self, kind: SymbolKind) -> Vec<Resolved> {
        let mut found: Vec<Resolved> = (self.checker.tables.iter())
            .flat_map(|(context, table)| {
                table
                    .symbols
                    .iter()
                    .filter(|(_, s)| s.kind == kind)
                    .map(move |(name, _)| Resolved {
                        context: context.clone(),
                        name: name.clone(),
                        kind,
                    })
            })
            .collect();
        found.sort_by(|a, b| (&a.context, &a.name).cmp(&(&b.context, &b.name)));
        found
    }
}
