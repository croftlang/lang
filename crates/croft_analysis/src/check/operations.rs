//! What an adapter must provide: the operations of a port, or of a
//! repository for an adapter written for the repository itself.

use croft_syntax::ast::{ConstraintUse, FunctionDecl};

use super::{Checker, Resolved, Scope};
use crate::symbols::SymbolKind;
use crate::types::Type;

/// One operation an adapter provides, with its types worked out.
#[derive(Debug, Clone)]
pub struct Operation<'a> {
    pub name: String,
    pub parameters: Vec<(String, Type, &'a [ConstraintUse])>,
    pub result: Type,
    pub result_constraints: &'a [ConstraintUse],
}

impl<'a> Checker<'a> {
    /// The operations an adapter for `port` provides. `port` is a port or
    /// a repository.
    pub(crate) fn operations_of(&self, port: &Resolved) -> Vec<Operation<'a>> {
        if port.kind == SymbolKind::Repository {
            return self.repository_operations(port);
        }
        let scope = self.scope_of(port);
        let Some(decl) = scope
            .context
            .ports
            .iter()
            .find(|p| p.name.name == port.name)
        else {
            return Vec::new();
        };
        self.declared_operations(scope, &decl.operations)
    }

    /// Operations as a port or repository declares them.
    fn declared_operations(
        &self,
        scope: Scope<'a>,
        operations: &'a [FunctionDecl],
    ) -> Vec<Operation<'a>> {
        (operations.iter())
            .map(|operation| Operation {
                name: operation.name.name.clone(),
                parameters: (operation.parameters.iter().flatten())
                    .map(|p| {
                        let ty = self.parameter_type(&scope, p);
                        (p.name.name.clone(), ty, p.constraints.as_slice())
                    })
                    .collect(),
                result: (operation.return_type.as_ref())
                    .map_or(Type::Unit, |ty| self.lower_type(&scope, ty)),
                result_constraints: operation.return_constraints.as_slice(),
            })
            .collect()
    }

    /// The operations a repository declares. With none declared, `find`
    /// by identity, failing with the aggregate's errors or a `StorageError`,
    /// and `save`, failing with a `StorageError`.
    fn repository_operations(&self, repository: &Resolved) -> Vec<Operation<'a>> {
        let scope = self.scope_of(repository);
        let decl = (scope.context.repositories.iter()).find(|r| r.name.name == repository.name);
        if let Some(operations) = decl.and_then(|r| r.operations.as_ref()) {
            return self.declared_operations(scope, operations);
        }
        let Some(aggregate) = self.repository_aggregate(repository) else {
            return Vec::new();
        };
        let errors = self
            .lookup(&aggregate.context, &format!("{}.Error", aggregate.name))
            .map_or(Type::Unknown, Type::Named);
        let storage = self.storage_error();
        let aggregate_type = Type::Named(aggregate.clone());
        vec![
            Operation {
                name: "find".to_string(),
                parameters: vec![("id".to_string(), self.identity_of(&aggregate), &[])],
                result: Type::result(
                    aggregate_type.clone(),
                    Type::Errors(vec![errors, storage.clone()]),
                ),
                result_constraints: &[],
            },
            Operation {
                name: "save".to_string(),
                parameters: vec![("value".to_string(), aggregate_type, &[])],
                result: Type::result(Type::Unit, storage),
                result_constraints: &[],
            },
        ]
    }
}
