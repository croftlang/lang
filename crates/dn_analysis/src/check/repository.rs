//! What repositories store, and what saving through them can fail with.

use super::{Checker, Resolved};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// The aggregate a repository stores.
    pub(super) fn repository_aggregate(&self, repository: &Resolved) -> Option<Resolved> {
        let scope = self.scope_of(repository);
        let decl = scope
            .context
            .repositories
            .iter()
            .find(|r| r.name.name == repository.name)?;
        self.resolve(&scope, &decl.aggregate)
    }

    /// The repository that stores an aggregate, if its context has one.
    pub(super) fn repository_for(&self, aggregate: &Resolved) -> Option<Resolved> {
        let context = &self.program.contexts[&aggregate.context];
        context
            .repositories
            .iter()
            .filter_map(|r| self.lookup(&aggregate.context, &r.name.name))
            .find(|r| self.repository_aggregate(r).as_ref() == Some(aggregate))
    }

    /// What saving fails with: the error of the storage port's first operation
    /// that returns a `Result`.
    pub(super) fn storage_error(&self, repository: &Resolved) -> Type {
        let scope = self.scope_of(repository);
        let decl = scope
            .context
            .repositories
            .iter()
            .find(|r| r.name.name == repository.name);
        let storage = decl
            .and_then(|r| r.storage.as_ref())
            .and_then(|path| self.resolve(&scope, path));
        let Some(storage) = storage.filter(|s| s.kind == SymbolKind::Port) else {
            return Type::Unknown;
        };
        let port_scope = self.scope_of(&storage);
        let port = port_scope
            .context
            .ports
            .iter()
            .find(|p| p.name.name == storage.name);
        let results = port
            .into_iter()
            .flat_map(|p| &p.operations)
            .filter_map(|op| op.return_type.as_ref());
        for ty in results {
            if let Type::Result(_, error) = self.lower_type(&port_scope, ty) {
                return *error;
            }
        }
        Type::Unknown
    }
}
