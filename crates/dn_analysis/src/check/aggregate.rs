//! Aggregates: their root fields, entities, invariants, errors, events and commands.

use dn_syntax::ast::AggregateDecl;

use super::types::is_builtin_type;
use super::{Checker, Scope};
use crate::program::Context;
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    pub(super) fn check_aggregate(&mut self, context: &'a Context, aggregate: &'a AggregateDecl) {
        let scope = Scope {
            context,
            aggregate: Some(&aggregate.name.name),
        };
        self.check_fields(&scope, &aggregate.fields);
        self.check_aggregate_identity(&scope, aggregate);
        for entity in &aggregate.entities {
            self.check_value(&scope, entity, true);
        }
        for invariant in &aggregate.invariants {
            self.check_invariant(&scope, invariant);
        }
        for record in aggregate.errors.iter().chain(&aggregate.events) {
            self.check_fields(&scope, &record.fields);
        }
        for command in &aggregate.commands {
            self.check_function(&scope, command);
        }
    }

    /// An aggregate root has exactly one identity field, of an identity type.
    fn check_aggregate_identity(&mut self, scope: &Scope, aggregate: &AggregateDecl) {
        let identities: Vec<_> = aggregate
            .fields
            .iter()
            .filter(|field| field.is_identity)
            .collect();
        let [identity] = identities[..] else {
            let message = format!(
                "aggregate `{}` needs exactly one `identity` field",
                aggregate.name.name
            );
            self.error(message, aggregate.name.span);
            return;
        };
        let name = identity.ty.path.text();
        let kind = self
            .resolve(scope, &identity.ty.path)
            .map(|found| found.kind);
        let unknown = kind.is_none() && !is_builtin_type(&name);
        if !unknown && kind != Some(SymbolKind::Identity) {
            let message = format!(
                "the identity of `{}` must be an identity type, not `{name}`",
                aggregate.name.name
            );
            self.error(message, identity.ty.path.span());
        }
    }
}
