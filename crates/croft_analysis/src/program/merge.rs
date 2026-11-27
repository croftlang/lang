//! Joining the parts of a context, and of an aggregate, written in different files.

use croft_syntax::ast::*;

use super::Context;

impl Context {
    /// An empty context named `name`.
    pub(super) fn new(name: Identifier) -> Context {
        Context {
            doc: Vec::new(),
            name,
            publish: Vec::new(),
            uses: Vec::new(),
            values: Vec::new(),
            enums: Vec::new(),
            errors: Vec::new(),
            consts: Vec::new(),
            identities: Vec::new(),
            constraints: Vec::new(),
            aggregates: Vec::new(),
            functions: Vec::new(),
            policies: Vec::new(),
            ports: Vec::new(),
            adapters: Vec::new(),
            repositories: Vec::new(),
            is_domain: false,
            is_standard: false,
        }
    }

    /// Adds one part's declarations, joining each aggregate to any of the same name.
    pub(super) fn add_items(&mut self, items: Vec<ContextItem>) {
        for item in items {
            match item {
                ContextItem::Publish(paths) => self.publish.extend(paths),
                ContextItem::Use(use_decl) => self.uses.push(use_decl),
                ContextItem::Value(value) => self.values.push(value),
                ContextItem::Enum(enum_decl) => self.enums.push(enum_decl),
                ContextItem::Error(error) => self.errors.push(error),
                ContextItem::Const(constant) => self.consts.push(constant),
                ContextItem::Identity(identity) => self.identities.push(identity),
                ContextItem::Constraint(constraint) => self.constraints.push(constraint),
                ContextItem::Aggregate(part) => self.add_aggregate(part),
                ContextItem::Function(function) => self.functions.push(function),
                ContextItem::Policy(policy) => self.policies.push(policy),
                ContextItem::Port(port) => self.ports.push(port),
                ContextItem::Adapter(adapter) => self.adapters.push(adapter),
                ContextItem::Repository(repository) => self.repositories.push(repository),
            }
        }
    }

    /// Adds an aggregate, or joins it to one of the same name from another file.
    fn add_aggregate(&mut self, part: AggregateDecl) {
        let existing = self
            .aggregates
            .iter_mut()
            .find(|a| a.name.name == part.name.name);

        let Some(aggregate) = existing else {
            self.aggregates.push(part);
            return;
        };

        if aggregate.doc.is_empty() {
            aggregate.doc = part.doc;
        }

        aggregate.fields.extend(part.fields);
        aggregate.invariants.extend(part.invariants);
        aggregate.entities.extend(part.entities);
        aggregate.errors.extend(part.errors);
        aggregate.events.extend(part.events);
        aggregate.commands.extend(part.commands);
        aggregate.factories.extend(part.factories);
    }

    /// The domain as a context of its own: it imports what it uses, and its
    /// body is a service named after it, so it is checked and compiled like one.
    pub(super) fn from_domain(domain: &DomainDecl) -> Context {
        let mut context = Context::new(domain.name.clone());
        context.is_domain = true;
        context.doc = domain.doc.clone();
        context.uses = domain.uses.clone();

        context.functions.push(FunctionDecl {
            type_parameters: Vec::new(),
            doc: domain.doc.clone(),
            kind: FunctionKind::Domain,
            name: domain.name.clone(),
            parameters: Some(Vec::new()),
            return_type: None,
            return_constraints: Vec::new(),
            preconditions: Vec::new(),
            calls: domain.calls.clone(),
            body: Some(domain.body.clone()),
        });

        context
    }
}
