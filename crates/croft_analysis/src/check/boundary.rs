//! Ports and repositories.

use std::collections::HashSet;

use croft_syntax::ast::{ExprKind, FunctionDecl, PortDecl, RepositoryDecl};

use super::constraints::ConstraintPlace;
use super::{Checker, Resolved, Scope};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// A port declares operations by signature only, each with a contract
    /// its callers keep, such as `low <= high`.
    pub(super) fn check_port(&mut self, scope: &Scope, port: &PortDecl) {
        self.check_operations(scope, &port.operations, "a port");

        for operation in &port.operations {
            for line in &operation.preconditions {
                let ExprKind::OrElse(_, error) = &line.kind else {
                    continue;
                };

                let message = format!(
                    "a port's contract line must hold, so `{}` can't fail with `{}`; give it a `Result` its adapter fails with",
                    operation.name.name,
                    error.text()
                );

                self.error(message, line.span);
            }
        }
    }

    /// Operations declared by signature only, in a port or repository. Only
    /// a port's can have a contract.
    fn check_operations(&mut self, scope: &Scope, operations: &[FunctionDecl], owner: &str) {
        let mut seen = HashSet::new();

        for operation in operations {
            let name = &operation.name;

            if !seen.insert(name.name.as_str()) {
                self.error(
                    format!("operation `{}` is declared more than once", name.name),
                    name.span,
                );
            }

            match &operation.parameters {
                Some(parameters) => {
                    self.check_parameters(scope, parameters, false, ConstraintPlace::Declaration)
                }
                None => self.error(
                    format!("`{}` needs brackets, even with no parameters", name.name),
                    name.span,
                ),
            }

            // With no return type, an operation gives back `Unit`.
            if let Some(ty) = &operation.return_type {
                self.check_type(scope, ty, false);
            }

            let contract = !operation.preconditions.is_empty() && owner != "a port";

            if operation.body.is_some() || contract {
                let message = format!(
                    "{owner} only declares `{}`; an adapter gives it a body",
                    name.name
                );

                self.error(message, name.span);
            }
        }
    }

    /// `repository Rooms for Room`: an aggregate of this context, kept by
    /// the adapter croft.toml chooses. Each operation it declares works on
    /// that aggregate alone, and can fail with `StorageError`.
    pub(super) fn check_repository(&mut self, scope: &Scope, repository: &RepositoryDecl) {
        let kinds = [SymbolKind::Aggregate];
        let aggregate = self.expect_kind(scope, &repository.aggregate, &kinds, "an aggregate");

        if let Some(found) = &aggregate {
            if found.context != scope.context.name.name {
                let message = "a repository can only store an aggregate of its own context";
                self.error(message, repository.aggregate.span());
            }
        }

        let Some(declared) = &repository.operations else {
            return;
        };

        if declared.is_empty() {
            let message = format!(
                "repository `{}` declares no operations; leave out the braces for `find` and `save`",
                repository.name.name
            );

            self.error(message, repository.name.span);
            return;
        }

        self.check_operations(scope, declared, "a repository");

        let (Some(aggregate), Some(resolved)) = (
            aggregate,
            self.lookup(&scope.context.name.name, &repository.name.name),
        ) else {
            return;
        };

        let storage = self.storage_error();

        for (decl, operation) in declared.iter().zip(self.operations_of(&resolved)) {
            let name = &decl.name;

            let fails_with_storage = match &operation.result {
                Type::Result(_, error) => self.accepts(error, &storage),
                _ => false,
            };

            if !fails_with_storage {
                let message = format!(
                    "`{}` can fail with `StorageError`, whichever adapter keeps the aggregates: `Result<..., StorageError>`",
                    name.name
                );

                self.error(message, name.span);
            }

            let types = (operation.parameters.iter().map(|(_, ty, _)| ty))
                .chain(std::iter::once(&operation.result));

            for ty in types {
                if let Some(other) = other_aggregate(ty, &aggregate) {
                    let message = format!(
                        "repository `{}` keeps `{}`, so `{}` can't take or give back `{}`",
                        repository.name.name, aggregate.name, name.name, other.name
                    );

                    self.error(message, name.span);
                }
            }
        }
    }
}

/// An aggregate other than `own` that a type holds, if there is one.
fn other_aggregate<'t>(ty: &'t Type, own: &Resolved) -> Option<&'t Resolved> {
    match ty {
        Type::Named(named) if named.kind == SymbolKind::Aggregate && named != own => Some(named),
        Type::List(item) | Type::Set(item) | Type::Option(item) => other_aggregate(item, own),
        Type::Map(key, value) | Type::Result(key, value) => {
            other_aggregate(key, own).or_else(|| other_aggregate(value, own))
        }
        Type::Errors(errors) => errors.iter().find_map(|e| other_aggregate(e, own)),
        _ => None,
    }
}
