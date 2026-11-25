//! Ports and repositories.

use std::collections::HashSet;

use croft_syntax::ast::{PortDecl, RepositoryDecl};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// A port declares operations by signature only.
    pub(super) fn check_port(&mut self, scope: &Scope, port: &PortDecl) {
        let mut seen = HashSet::new();
        for operation in &port.operations {
            let name = &operation.name;
            if !seen.insert(name.name.as_str()) {
                self.error(
                    format!("operation `{}` is declared more than once", name.name),
                    name.span,
                );
            }
            match &operation.parameters {
                Some(parameters) => self.check_parameters(scope, parameters, false),
                None => self.error(
                    format!("`{}` needs brackets, even with no parameters", name.name),
                    name.span,
                ),
            }
            // With no return type, an operation gives back `Unit`.
            if let Some(ty) = &operation.return_type {
                self.check_type(scope, ty, false);
            }
            if operation.body.is_some() || !operation.preconditions.is_empty() {
                let message = format!(
                    "a port only declares `{}`; an adapter gives it a body",
                    name.name
                );
                self.error(message, name.span);
            }
        }
    }

    /// `repository Rooms for Room`: an aggregate of this context, kept in
    /// the storage croft.toml chooses.
    pub(super) fn check_repository(&mut self, scope: &Scope, repository: &RepositoryDecl) {
        let kinds = [SymbolKind::Aggregate];
        if let Some(found) = self.expect_kind(scope, &repository.aggregate, &kinds, "an aggregate")
        {
            if found.context != scope.context.name.name {
                let message = "a repository can only store an aggregate of its own context";
                self.error(message, repository.aggregate.span());
            }
        }
    }
}
