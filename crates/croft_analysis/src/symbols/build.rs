use croft_syntax::ast::{FunctionKind, Identifier};
use croft_syntax::Diagnostic;

use super::{SymbolKind, SymbolTable};
use crate::program::Context;

/// Lists every name a context declares, reporting names declared twice.
pub fn build_table(context: &Context, errors: &mut Vec<Diagnostic>) -> SymbolTable {
    let mut table = SymbolTable::default();
    let mut add = |names: Vec<&Identifier>, kind| table.add_all(names, "", kind, errors);

    add(
        context.values.iter().map(|v| &v.name).collect(),
        SymbolKind::Value,
    );

    add(
        context.enums.iter().map(|e| &e.name).collect(),
        SymbolKind::Enum,
    );

    add(
        context.errors.iter().map(|e| &e.name).collect(),
        SymbolKind::Error,
    );

    add(
        context.consts.iter().map(|c| &c.name).collect(),
        SymbolKind::Const,
    );

    add(
        context.identities.iter().map(|i| &i.name).collect(),
        SymbolKind::Identity,
    );

    add(
        context.constraints.iter().map(|c| &c.name).collect(),
        SymbolKind::Constraint,
    );

    add(
        context.aggregates.iter().map(|a| &a.name).collect(),
        SymbolKind::Aggregate,
    );

    add(
        context.policies.iter().map(|p| &p.name).collect(),
        SymbolKind::Policy,
    );

    add(
        context.ports.iter().map(|p| &p.name).collect(),
        SymbolKind::Port,
    );

    add(
        context.adapters.iter().map(|a| &a.name).collect(),
        SymbolKind::Adapter,
    );

    add(
        context.repositories.iter().map(|r| &r.name).collect(),
        SymbolKind::Repository,
    );

    for function in &context.functions {
        let kind = match function.kind {
            FunctionKind::Service | FunctionKind::Domain => SymbolKind::Service,
            FunctionKind::Translation => SymbolKind::Translation,
            _ => SymbolKind::Function,
        };

        add(vec![&function.name], kind);
    }

    for aggregate in &context.aggregates {
        let prefix = format!("{}.", aggregate.name.name);
        let entities = aggregate.entities.iter().map(|e| &e.name);
        table.add_all(entities, &prefix, SymbolKind::Entity, errors);

        table.add_all(
            aggregate.errors.iter().map(|e| &e.name),
            &prefix,
            SymbolKind::Error,
            errors,
        );

        table.add_all(
            aggregate.events.iter().map(|e| &e.name),
            &prefix,
            SymbolKind::Event,
            errors,
        );

        let commands = aggregate.commands.iter().map(|c| &c.name);
        table.add_all(commands, &prefix, SymbolKind::Command, errors);

        let error_set = format!("{prefix}Error");
        table.add(error_set, SymbolKind::ErrorSet, aggregate.name.span, errors);
    }

    table
}
