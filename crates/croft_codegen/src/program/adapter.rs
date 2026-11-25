//! The adapters this target uses. An adapter's fields live in one global
//! struct, which its operations read and `set`.

use croft_analysis::types::Resolved;
use croft_analysis::Type;

use super::super::types::define::struct_definition;
use super::constraints::CheckPlace;
use crate::function::{OwnFields, Returns, Writer};
use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The adapters' state structs, and their operations.
    pub fn adapters(&self, prototypes: &mut Vec<String>) -> (String, String) {
        let (mut state, mut out) = (String::new(), String::new());

        let mut bound: Vec<(Resolved, Resolved)> = (self.bindings.iter())
            .map(|(port, adapter)| (port.clone(), adapter.clone()))
            .collect();

        let storage = self.model.lookup("Std.Storage", "Storage");

        for (repository, adapter) in &self.storages {
            if self.is_repository_adapter(repository, adapter) {
                bound.push((repository.clone(), adapter.clone()));
            } else if let Some(storage) = &storage {
                bound.push((storage.clone(), adapter.clone()));
            }
        }

        bound.sort_by_key(|(_, adapter)| c_name(adapter));
        bound.dedup_by(|a, b| a.1 == b.1);

        let bound = bound.iter().map(|(port, adapter)| (port, adapter));

        for (port, adapter) in bound {
            let context = &self.program.contexts[&adapter.context];

            let Some(decl) = context
                .adapters
                .iter()
                .find(|a| a.name.name == adapter.name)
            else {
                continue;
            };

            let name = c_name(adapter);

            let fields: Vec<(String, Type)> = decl
                .fields
                .iter()
                .map(|f| {
                    (
                        f.name.name.clone(),
                        self.model.lower_type(&adapter.context, None, &f.ty),
                    )
                })
                .collect();

            if !fields.is_empty() {
                let members: String = fields
                    .iter()
                    .map(|(f, ty)| format!("    {} {f};\n", self.c_type(ty)))
                    .collect();

                state.push_str(&struct_definition(&format!("{name}_fields"), &members));
                state.push_str(&format!("{name}_fields {name}_state;\n\n"));
            }

            let expected_operations = self.model.operations_of(port);

            for operation in &decl.operations {
                let Some(expected) = expected_operations
                    .iter()
                    .find(|op| op.name == operation.name.name)
                else {
                    continue;
                };

                let result = self.operation_result(port, &operation.name.name);

                let parameters: Vec<(String, Type)> = (expected.parameters.iter())
                    .map(|(name, ty, _)| (name.clone(), ty.clone()))
                    .collect();

                let signature = self.signature(
                    &format!("{name}_{}", operation.name.name),
                    &[],
                    &parameters,
                    Some(&result),
                );

                prototypes.push(format!("{signature};"));

                let mut writer =
                    Writer::new(self, &adapter.context, Returns::Value(result.clone()));

                writer.own = Some(OwnFields {
                    prefix: format!("{name}_state."),
                    fields: fields.clone(),
                    owner: None,
                });

                let names: Vec<String> = operation
                    .parameters
                    .iter()
                    .flatten()
                    .map(|p| p.name.name.clone())
                    .collect();

                let renamed: Vec<(String, Type)> = names
                    .into_iter()
                    .zip(parameters.iter().map(|(_, ty)| ty.clone()))
                    .collect();

                writer.bind_parameters(&renamed);

                let function = format!("{name}_{}", operation.name.name);
                let signature = self.signature(&function, &[], &renamed, Some(&result));
                let label = format!("{}.{}", adapter.name, operation.name.name);

                let place = CheckPlace {
                    context: &port.context,
                    aggregate: None,
                    label: &label,
                };

                let constraints = expected
                    .parameters
                    .iter()
                    .map(|(_, _, constraints)| *constraints);

                let checked: Vec<(String, Type, &[croft_syntax::ast::ConstraintUse])> = renamed
                    .iter()
                    .zip(constraints)
                    .map(|((n, ty), constraints)| (n.clone(), ty.clone(), constraints))
                    .collect();

                if let Some(call) = self.argument_check(&function, &place, &checked) {
                    writer.debug_call(&call);
                }

                if let Some(body) = &operation.body {
                    let value = writer.value_into("(*out)", body, &result);
                    if let Some(check) =
                        self.result_check(&function, &place, &result, expected.result_constraints)
                    {
                        let passed = writer.pass(value.clone(), &result);
                        writer.debug_call(&format!("{check}({passed});"));
                    }
                }

                out.push_str(&writer.finish(&signature));
            }
        }

        (state, out)
    }

    /// Whether `adapter` was written for `repository` itself, giving the
    /// operations it declares, rather than for `Storage`.
    pub fn is_repository_adapter(&self, repository: &Resolved, adapter: &Resolved) -> bool {
        let context = &self.program.contexts[&adapter.context];

        let decl = context
            .adapters
            .iter()
            .find(|a| a.name.name == adapter.name);

        let port = decl.and_then(|decl| self.model.resolve(&adapter.context, None, &decl.port));

        port.as_ref() == Some(repository)
    }
}
