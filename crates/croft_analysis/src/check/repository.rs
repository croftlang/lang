//! What repositories store, what saving through them can fail with, and
//! which of their operations `Storage` can provide.

use super::operations::Operation;
use super::{Checker, Resolved};
use crate::standard::STORAGE_CONTEXT;
use crate::types::Type;

/// A repository operation `Storage` can provide, told apart by its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageShape {
    /// `fn find(id: RoomId): Result<Room, ...>`, failing with the
    /// aggregate's `NotFound` when nothing is kept under the identity.
    Find,
    /// `fn find(id: RoomId): Result<Option<Room>, ...>`
    FindOption,
    /// `fn save(room: Room): Result<Unit, ...>`
    Save,
    /// `fn remove(id: RoomId): Result<Unit, ...>`
    Remove,
}

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

    /// Which shape an operation of the repository for `aggregate` has, if
    /// `Storage` can provide it.
    pub(super) fn storage_shape(
        &self,
        aggregate: &Resolved,
        operation: &Operation,
    ) -> Option<StorageShape> {
        let [(_, parameter, _)] = operation.parameters.as_slice() else {
            return None;
        };

        let Type::Result(value, error) = &operation.result else {
            return None;
        };

        let aggregate_type = Type::Named(aggregate.clone());

        if *parameter == aggregate_type {
            return (**value == Type::Unit).then_some(StorageShape::Save);
        }

        if *parameter != self.identity_of(aggregate) {
            return None;
        }

        match &**value {
            Type::Unit => Some(StorageShape::Remove),
            Type::Option(item) if **item == aggregate_type => Some(StorageShape::FindOption),
            found if *found == aggregate_type => {
                // An aggregate with no `NotFound` fails with its error set,
                // as `find` does in a repository that declares nothing.
                let name = |error: &str| format!("{}.{error}", aggregate.name);
                let missing = (self.lookup(&aggregate.context, &name("NotFound")))
                    .or_else(|| self.lookup(&aggregate.context, &name("Error")));
                let fails = missing.is_some_and(|e| self.accepts(error, &Type::Named(e)));
                fails.then_some(StorageShape::Find)
            }
            _ => None,
        }
    }

    /// The operations `issue ... on Room(id)` loads and saves through: a
    /// find that fails with `NotFound`, and a save.
    pub(super) fn find_and_save(
        &self,
        repository: &Resolved,
    ) -> Option<(Operation<'a>, Operation<'a>)> {
        let aggregate = self.repository_aggregate(repository)?;
        let operations = self.operations_of(repository);

        let with_shape = |shape| {
            (operations.iter())
                .find(|op| self.storage_shape(&aggregate, op) == Some(shape))
                .cloned()
        };

        Some((
            with_shape(StorageShape::Find)?,
            with_shape(StorageShape::Save)?,
        ))
    }
}
