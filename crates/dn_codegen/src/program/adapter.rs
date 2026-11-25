//! The adapters this target uses. An adapter's fields live in one global
//! struct, which its operations read and `set`.

use dn_analysis::Type;

use super::super::types::define::struct_definition;
use super::constraints::CheckPlace;
use crate::function::{OwnFields, Returns, Writer};
use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The adapters' state structs, and their operations.
    pub fn adapters(&self, prototypes: &mut Vec<String>) -> (String, String) {
        let (mut state, mut out) = (String::new(), String::new());
        let mut bound: Vec<_> = self.bindings.iter().collect();
        bound.sort_by_key(|(_, adapter)| c_name(adapter));
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
            let port_context = &self.program.contexts[&port.context];
            let Some(port_decl) = port_context.ports.iter().find(|p| p.name.name == port.name)
            else {
                continue;
            };
            for operation in &decl.operations {
                let Some(expected) = port_decl
                    .operations
                    .iter()
                    .find(|op| op.name.name == operation.name.name)
                else {
                    continue;
                };
                let result = expected.return_type.as_ref().map_or(Type::Unit, |ty| {
                    self.model.lower_type(&port.context, None, ty)
                });
                let parameters = self.operation_parameters(port, &operation.name.name);
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
                    .flatten()
                    .map(|p| p.constraints.as_slice());
                let checked: Vec<(String, Type, &[dn_syntax::ast::ConstraintUse])> = renamed
                    .iter()
                    .zip(constraints)
                    .map(|((n, ty), constraints)| (n.clone(), ty.clone(), constraints))
                    .collect();
                if let Some(call) = self.argument_check(&function, &place, &checked) {
                    writer.debug_call(&call);
                }
                if let Some(body) = &operation.body {
                    let value = writer.value(body, &result);
                    if let Some(check) =
                        self.result_check(&function, &place, &result, &expected.return_constraints)
                    {
                        let passed = writer.pass(value.clone(), &result);
                        writer.debug_call(&format!("{check}({passed});"));
                    }
                    writer.line(format!("*out = {value};"));
                }
                out.push_str(&writer.finish(&signature));
            }
        }
        (state, out)
    }
}
