//! Adapter operations, whose parameter and return types come from their port.

use dn_syntax::ast::AdapterDecl;

use super::{BodyChecker, BodyKind};
use crate::check::{Checker, Scope};
use crate::types::Type;

impl<'a> Checker<'a> {
    /// An adapter operation's parameter and return types come from its port.
    pub(super) fn check_adapter_bodies(&mut self, scope: Scope<'a>, adapter: &'a AdapterDecl) {
        let Some(port) = self.resolve(&scope, &adapter.port) else {
            return;
        };
        let port_scope = self.scope_of(&port);
        let ports = &port_scope.context.ports;
        let Some(port_decl) = ports.iter().find(|p| p.name.name == port.name) else {
            return;
        };
        for operation in &adapter.operations {
            let Some(expected) = port_decl
                .operations
                .iter()
                .find(|op| op.name.name == operation.name.name)
            else {
                continue;
            };
            let result = expected
                .return_type
                .as_ref()
                .map_or(Type::Unit, |ty| self.lower_type(&port_scope, ty));
            let parameters: Vec<(String, Type)> = (expected.parameters.iter().flatten())
                .map(|p| (p.name.name.clone(), self.parameter_type(&port_scope, p)))
                .collect();
            let mut body = BodyChecker::new(self, scope, BodyKind::Adapter(adapter), result);
            let names = operation
                .parameters
                .iter()
                .flatten()
                .map(|p| p.name.name.clone());
            for (name, (_, ty)) in names.zip(parameters) {
                body.bind(&name, ty, false);
            }
            if let Some(value) = &operation.body {
                body.expect_body(value);
            }
        }
    }
}
