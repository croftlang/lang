//! Adapters: one port's operations, written for one target.

use dn_syntax::ast::{AdapterDecl, Field, FunctionDecl};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// An adapter provides every operation of its port, and nothing else.
    pub(super) fn check_adapter(&mut self, scope: &Scope, adapter: &AdapterDecl) {
        let without_values: Vec<_> = adapter
            .fields
            .iter()
            .map(|f| Field {
                value: None,
                ..f.clone()
            })
            .collect();
        self.check_fields(scope, &without_values);
        for field in adapter.fields.iter().filter(|f| f.value.is_none()) {
            let message = format!(
                "adapter field `{}` needs a starting value: `{}: ... = ...`",
                field.name.name, field.name.name
            );
            self.error(message, field.name.span);
        }
        let Some(found) = self.expect_kind(scope, &adapter.port, &[SymbolKind::Port], "a port")
        else {
            return;
        };
        let ports = &self.program.contexts[&found.context].ports;
        let port = ports
            .iter()
            .find(|port| port.name.name == found.name)
            .expect("a resolved port");
        for operation in &adapter.operations {
            let expected = port
                .operations
                .iter()
                .find(|op| op.name.name == operation.name.name);
            match expected {
                Some(expected) => {
                    self.check_adapter_operation(operation, expected, &port.name.name)
                }
                None => {
                    let message = format!(
                        "port `{}` has no operation `{}`",
                        port.name.name, operation.name.name
                    );
                    self.error(message, operation.name.span);
                }
            }
        }
        for expected in &port.operations {
            if !adapter
                .operations
                .iter()
                .any(|op| op.name.name == expected.name.name)
            {
                let message = format!(
                    "adapter `{}` does not provide `{}` from port `{}`",
                    adapter.name.name, expected.name.name, port.name.name
                );
                self.error(message, adapter.name.span);
            }
        }
    }

    fn check_adapter_operation(
        &mut self,
        operation: &FunctionDecl,
        expected: &FunctionDecl,
        port: &str,
    ) {
        let name = &operation.name;
        let count = |op: &FunctionDecl| op.parameters.as_ref().map_or(0, Vec::len);
        let (found, wanted) = (count(operation), count(expected));
        if found != wanted {
            let message = format!(
                "`{}` takes {wanted} parameters in port `{port}`, found {found}",
                name.name
            );
            self.error(message, name.span);
        }
        if operation.body.is_none() {
            self.error(format!("`{}` needs a body after `=`", name.name), name.span);
        }
    }
}
