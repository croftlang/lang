//! Adapters: one port's operations, written for one target.

use croft_syntax::ast::{AdapterDecl, Field, FunctionDecl};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// An adapter provides every operation of its port, and nothing else.
    pub(super) fn check_adapter(&mut self, scope: &Scope<'a>, adapter: &AdapterDecl) {
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

        let kinds = [SymbolKind::Port, SymbolKind::Repository];

        let Some(found) = self.expect_kind(scope, &adapter.port, &kinds, "a port or repository")
        else {
            return;
        };

        let expected = self.operations_of(&found);

        let noun = if found.kind == SymbolKind::Repository {
            "repository"
        } else {
            "port"
        };

        for operation in &adapter.operations {
            match expected.iter().find(|op| op.name == operation.name.name) {
                Some(expected) => self.check_adapter_operation(
                    operation,
                    expected.parameters.len(),
                    &format!("{noun} `{}`", found.name),
                ),
                None => {
                    let message = format!(
                        "{noun} `{}` has no operation `{}`",
                        found.name, operation.name.name
                    );

                    self.error(message, operation.name.span);
                }
            }
        }

        for expected in &expected {
            if !adapter
                .operations
                .iter()
                .any(|op| op.name.name == expected.name)
            {
                let message = format!(
                    "adapter `{}` does not provide `{}` from {noun} `{}`",
                    adapter.name.name, expected.name, found.name
                );

                self.error(message, adapter.name.span);
            }
        }
    }

    /// An adapter operation takes as many parameters as the port's operation,
    /// `wanted`, and has a body.
    fn check_adapter_operation(&mut self, operation: &FunctionDecl, wanted: usize, port: &str) {
        let name = &operation.name;
        let found = operation.parameters.as_ref().map_or(0, Vec::len);

        if found != wanted {
            let message = format!(
                "`{}` takes {wanted} parameters in {port}, found {found}",
                name.name
            );

            self.error(message, name.span);
        }

        if operation.body.is_none() {
            self.error(format!("`{}` needs a body after `=`", name.name), name.span);
        }
    }
}
