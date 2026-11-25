//! Adapters: their fields' starting values, and their operations, whose
//! parameter and return types come from their port.

use b_syntax::ast::AdapterDecl;

use super::{BodyChecker, BodyKind};
use crate::check::{Checker, Scope};
use crate::types::Type;

impl<'a> Checker<'a> {
    /// An adapter operation's parameter and return types come from its port.
    pub(super) fn check_adapter_bodies(&mut self, scope: Scope<'a>, adapter: &'a AdapterDecl) {
        for field in &adapter.fields {
            let Some(value) = &field.value else { continue };
            let ty = self.lower_type(&scope, &field.ty);
            BodyChecker::new(self, scope, BodyKind::Constant, ty.clone()).expect(value, &ty);
        }
        let Some(port) = self.resolve(&scope, &adapter.port) else {
            return;
        };
        let expected_operations = self.operations_of(&port);
        for operation in &adapter.operations {
            let Some(expected) = expected_operations
                .iter()
                .find(|op| op.name == operation.name.name)
            else {
                continue;
            };
            let result = expected.result.clone();
            let parameters: Vec<(String, Type)> = (expected.parameters.iter())
                .map(|(name, ty, _)| (name.clone(), ty.clone()))
                .collect();
            let mut body = BodyChecker::new(self, scope, BodyKind::Adapter(adapter), result);
            let names = operation
                .parameters
                .iter()
                .flatten()
                .map(|p| (p.name.name.clone(), p.name.span));
            for ((name, span), (_, ty)) in names.zip(parameters) {
                body.bind(&name, ty, false, Some(span));
            }
            if let Some(value) = &operation.body {
                body.expect_body(value);
            }
        }
    }
}
