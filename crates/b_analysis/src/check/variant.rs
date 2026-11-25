//! The variants an enum or error set can be matched against.

use b_syntax::ast::Field;

use super::{Checker, Resolved, Scope};
use crate::symbols::SymbolKind;

/// One variant a `match` can name: an enum's own variant, or an error it includes.
#[derive(Clone)]
pub(super) struct Variant<'a> {
    pub name: String,
    pub fields: &'a [Field],
    pub scope: Scope<'a>,
    /// The error this variant stands for, when it comes from an included error.
    pub error: Option<Resolved>,
}

impl<'a> Checker<'a> {
    /// The variants of an enum, with every included error or enum spread out.
    pub(super) fn variants_of(&self, named: &Resolved) -> Vec<Variant<'a>> {
        let mut variants = Vec::new();
        if named.kind == SymbolKind::Enum {
            let context = &self.program.contexts[&named.context];
            let Some(decl) = context.enums.iter().find(|e| e.name.name == named.name) else {
                return variants;
            };
            let scope = Scope {
                context,
                aggregate: None,
            };
            for variant in &decl.variants {
                variants.push(Variant {
                    name: variant.name.name.clone(),
                    fields: &variant.fields,
                    scope,
                    error: None,
                });
            }
            for include in &decl.includes {
                if let Some(included) = self.resolve(&scope, include) {
                    variants.extend(self.variants_of(&included));
                }
            }
        } else {
            for error in self.error_members(named) {
                let fields = self.fields_of(&error).unwrap_or(&[]);
                let name = error
                    .name
                    .rsplit('.')
                    .next()
                    .unwrap_or(&error.name)
                    .to_string();
                variants.push(Variant {
                    name,
                    fields,
                    scope: self.scope_of(&error),
                    error: Some(error.clone()),
                });
            }
        }
        variants
    }
}
