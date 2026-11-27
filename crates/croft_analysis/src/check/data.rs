//! Values, entities, enums and identities.

use std::collections::HashSet;

use croft_syntax::ast::{
    EnumDecl, Expr, ExprKind, Identifier, IdentityDecl, Invariant, Path, ValueDecl,
};

use super::constraints::ConstraintPlace;
use super::{Checker, Scope};
use crate::program::{identifies, Context};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// A value's fields and invariants. Making a value can't fail, so each
    /// invariant must hold and names no error.
    pub(super) fn check_value(&mut self, scope: &Scope<'a>, value: &ValueDecl) {
        self.check_fields(scope, &value.fields);

        for invariant in &value.invariants {
            self.check_constraint_use(scope, &invariant.constraint);

            if invariant.error.is_none() {
                continue;
            }

            let message = format!(
                "making a value can't fail, so its invariant must hold: `invariant {}`, without `or`",
                invariant.constraint.text()
            );

            self.error(message, invariant.constraint.name.span());
        }
    }

    /// An entity's fields and invariants. Its identity is checked with its
    /// aggregate's.
    pub(super) fn check_entity(&mut self, scope: &Scope<'a>, entity: &ValueDecl) {
        self.check_fields(scope, &entity.fields);

        for invariant in &entity.invariants {
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

    /// An enum's variants are each declared once, and what it includes is an
    /// enum, an error or an error set.
    pub(super) fn check_enum(&mut self, scope: &Scope<'a>, enum_decl: &EnumDecl) {
        let mut seen = HashSet::new();

        for variant in &enum_decl.variants {
            if !seen.insert(variant.name.name.as_str()) {
                let message = format!("variant `{}` is declared more than once", variant.name.name);
                self.error(message, variant.name.span);
            }

            self.check_fields(scope, &variant.fields);

            for value in variant.fields.iter().filter_map(|f| f.value.as_ref()) {
                let message =
                    "a variant's fields have no starting values, since they are given like arguments";
                self.error(message, value.span);
            }
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

        let place = ConstraintPlace::Identity;
        self.check_constraints(scope, &identity.constraints, Some(&identity.ty), place);
        self.check_identity_owner(scope, identity);
    }

    /// `for` names one aggregate or entity in the identity's own context, and
    /// no other identity is for it too.
    fn check_identity_owner(&mut self, scope: &Scope, identity: &IdentityDecl) {
        let context = scope.context;
        let written = identity.owner.text();
        let span = identity.owner.span();
        let owners = owners_named(context, &identity.owner);

        match owners.as_slice() {
            [] => {
                let is_value = context.values.iter().any(|v| v.name.name == written);

                let message = if is_value {
                    format!("`{written}` is a value, which has no identity; make it an entity")
                } else {
                    format!(
                        "identity `{}` is for `{written}`, but `{}` has no aggregate or entity \
                         by that name",
                        identity.name.name, context.name.name
                    )
                };

                self.error(message, span);
            }
            [found] => {
                let first = context
                    .identities
                    .iter()
                    .find(|other| owners_named(context, &other.owner).contains(found));

                if let Some(first) = first.filter(|first| first.name.name != identity.name.name) {
                    let message = format!(
                        "`{found}` is already identified by `{}`; it has only one identity",
                        first.name.name
                    );

                    self.error(message, span);
                }
            }
            _ => {
                let message = format!(
                    "`{written}` could be {}; write which one, such as `{}`",
                    owners.join(" or "),
                    owners[0]
                );

                self.error(message, span);
            }
        }
    }
}

/// The aggregates and entities an identity's `for` could mean, written in
/// full: `Room`, `Room.Enemy`.
fn owners_named(context: &Context, owner: &Path) -> Vec<String> {
    let mut owners = Vec::new();

    for aggregate in &context.aggregates {
        let name = aggregate.name.name.as_str();

        if identifies(owner, name, None) {
            owners.push(name.to_string());
        }

        for entity in &aggregate.entities {
            if identifies(owner, name, Some(&entity.name.name)) {
                owners.push(format!("{name}.{}", entity.name.name));
            }
        }
    }

    owners
}

/// The name an expression spells, if it is just a name: `ExitBlocked`, `Room.NotFound`.
pub(super) fn expr_path(value: &Expr) -> Option<Path> {
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
