//! Aggregates: their root fields, entities, invariants, errors, events and commands.

use croft_syntax::ast::{AggregateDecl, Field, Identifier, IdentityDecl};

use super::{Checker, Scope};
use crate::program::Context;

impl<'a> Checker<'a> {
    pub(super) fn check_aggregate(&mut self, context: &'a Context, aggregate: &'a AggregateDecl) {
        let scope = Scope {
            context,
            aggregate: Some(&aggregate.name.name),
        };

        self.check_fields(&scope, &aggregate.fields);
        self.check_aggregate_identity(&scope, aggregate);

        for entity in &aggregate.entities {
            self.check_value(&scope, entity);
            let identity = context.identity_for(&aggregate.name.name, Some(&entity.name.name));
            self.check_identity_field("entity", &entity.name, &entity.fields, identity);
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

    /// An aggregate root has exactly one field of the identity declared for it.
    fn check_aggregate_identity(&mut self, scope: &Scope, aggregate: &AggregateDecl) {
        let identity = scope.context.identity_for(&aggregate.name.name, None);
        self.check_identity_field("aggregate", &aggregate.name, &aggregate.fields, identity);
    }

    /// An aggregate or entity holds its identity in exactly one field.
    pub(super) fn check_identity_field(
        &mut self,
        kind: &str,
        name: &Identifier,
        fields: &[Field],
        identity: Option<&IdentityDecl>,
    ) {
        let owner = &name.name;

        let Some(identity) = identity else {
            let message = format!(
                "{kind} `{owner}` has no identity; declare one in the context: \
                 `identity {owner}Id for {owner}: Int`"
            );

            self.error(message, name.span);
            return;
        };

        let id = &identity.name.name;
        let holding: Vec<_> = fields.iter().filter(|field| field.is_identity).collect();

        match holding[..] {
            [_] => {}
            [] => {
                let message = format!("{kind} `{owner}` needs a field of its identity: `id: {id}`");
                self.error(message, name.span);
            }
            [_, ref extra @ ..] => {
                for field in extra {
                    let message = format!(
                        "{kind} `{owner}` already has a field of its identity `{id}`; \
                         it holds only one"
                    );

                    self.error(message, field.name.span);
                }
            }
        }
    }
}
