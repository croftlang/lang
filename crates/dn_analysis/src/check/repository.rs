//! What repositories store, and what saving through them can fail with.

use super::{Checker, Resolved};
use crate::standard::STORAGE_CONTEXT;
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

    /// What loading and saving fail with besides the aggregate's own errors:
    /// the built-in `StorageError`, whichever storage the target chooses.
    pub(super) fn storage_error(&self) -> Type {
        self.lookup(STORAGE_CONTEXT, "StorageError")
            .map_or(Type::Unknown, Type::Named)
    }
}
