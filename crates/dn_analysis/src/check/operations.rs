//! What an adapter must provide: the operations of a port, or `find` and
//! `save` for an adapter written for a repository itself.

use dn_syntax::ast::ConstraintUse;

use super::{Checker, Resolved};
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
    /// The operations an adapter for `port` provides. `port` is a port, or
    /// a repository whose adapter writes `find` and `save` by hand.
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
        (decl.operations.iter())
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

    /// `find` by identity, failing with the aggregate's errors or a
    /// `StorageError`, and `save`, failing with a `StorageError`: the same
    /// as a repository the compiler writes, so services don't change.
    fn repository_operations(&self, repository: &Resolved) -> Vec<Operation<'a>> {
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
