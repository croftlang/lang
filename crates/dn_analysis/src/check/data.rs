//! Values, entities, enums and identities.

use std::collections::HashSet;

use dn_syntax::ast::{
    EnumDecl, Expr, ExprKind, Identifier, IdentityDecl, Invariant, Path, ValueDecl,
};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// A value has no identity; an entity has exactly one.
    pub(super) fn check_value(&mut self, scope: &Scope, value: &ValueDecl, is_entity: bool) {
        self.check_fields(scope, &value.fields);
        let identities: Vec<_> = value
            .fields
            .iter()
            .filter(|field| field.is_identity)
            .collect();
        let name = &value.name;
        if is_entity && identities.len() != 1 {
            let message = format!("entity `{}` needs exactly one `identity` field", name.name);
            self.error(message, name.span);
        }
        if !is_entity {
            for field in identities {
                let message = format!(
                    "value `{}` cannot have an identity; make it an entity",
                    name.name
                );
                self.error(message, field.name.span);
            }
        }
        for invariant in &value.invariants {
            self.check_invariant(scope, invariant);
        }
    }

    /// `invariant ExitClear or ExitBlocked`: a constraint, and the error it fails with.
    pub(super) fn check_invariant(&mut self, scope: &Scope, invariant: &Invariant) {
        self.check_constraint_use(scope, &invariant.constraint);
        let Some(error) = &invariant.error else {
            let message = "an invariant needs the error it fails with: `invariant Name or Error`";
            self.error(message, invariant.constraint.name.span());
            return;
        };
        match expr_path(error) {
            Some(path) => {
                self.expect_kind(scope, &path, &[SymbolKind::Error], "an error");
            }
            None => self.error("expected the name of an error", error.span),
        }
    }

    pub(super) fn check_enum(&mut self, scope: &Scope, enum_decl: &EnumDecl) {
        let mut seen = HashSet::new();
        for variant in &enum_decl.variants {
            if !seen.insert(variant.name.name.as_str()) {
                let message = format!("variant `{}` is declared more than once", variant.name.name);
                self.error(message, variant.name.span);
            }
            self.check_fields(scope, &variant.fields);
        }
        let kinds = [SymbolKind::Enum, SymbolKind::Error, SymbolKind::ErrorSet];
        for include in &enum_decl.includes {
            self.expect_kind(scope, include, &kinds, "an enum or errors to include");
        }
    }

    /// An identity is a distinct type built on `Int` or `String`.
    pub(super) fn check_identity(&mut self, scope: &Scope, identity: &IdentityDecl) {
        let base = identity.ty.path.text();
        if base != "Int" && base != "String" {
            let message = format!(
                "identity `{}` must be an `Int` or a `String`",
                identity.name.name
            );
            self.error(message, identity.ty.path.span());
        }
        self.check_rules(scope, &identity.rules, Some(&identity.ty));
    }
}

/// The name an expression spells, if it is just a name: `ExitBlocked`, `Room.NotFound`.
fn expr_path(value: &Expr) -> Option<Path> {
    match &value.kind {
        ExprKind::Name(name) => {
            let segment = Identifier {
                name: name.clone(),
                span: value.span,
            };
            Some(Path {
                segments: vec![segment],
            })
        }
        ExprKind::Field(inner, field) => {
            let mut path = expr_path(inner)?;
            path.segments.push(field.clone());
            Some(path)
        }
        _ => None,
    }
}
