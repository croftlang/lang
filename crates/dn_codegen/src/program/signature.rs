//! Parameters and C signatures of generated functions. Numbers are passed by
//! value, everything else by pointer, and every result through `out`.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::Parameter;

use crate::function::Writer;
use crate::Generator;

impl<'a> Generator<'a> {
    fn lower_parameters(
        &self,
        context: &str,
        aggregate: Option<&str>,
        parameters: &[Parameter],
    ) -> Vec<(String, Type)> {
        let lower = |p: &Parameter| {
            p.types.first().map_or(Type::Unknown, |ty| {
                self.model.lower_type(context, aggregate, ty)
            })
        };
        parameters
            .iter()
            .map(|p| (p.name.name.clone(), lower(p)))
            .collect()
    }

    /// Parameters of a function, translation, service or constraint declared in dn.
    pub fn parameters_of(&self, found: &Resolved) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&found.context];
        let parameters = if found.kind == SymbolKind::Constraint {
            context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name)
                .map(|c| c.parameters.as_slice())
        } else {
            context
                .functions
                .iter()
                .find(|f| f.name.name == found.name)
                .and_then(|f| f.parameters.as_deref())
        };
        self.lower_parameters(&found.context, None, parameters.unwrap_or(&[]))
    }

    pub fn operation_parameters(&self, port: &Resolved, operation: &str) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&port.context];
        let port_decl = context.ports.iter().find(|p| p.name.name == port.name);
        let operation =
            port_decl.and_then(|p| p.operations.iter().find(|op| op.name.name == operation));
        let parameters = operation
            .and_then(|op| op.parameters.as_deref())
            .unwrap_or(&[]);
        self.lower_parameters(&port.context, None, parameters)
    }

    pub fn command_parameters(&self, aggregate: &Resolved, command: &str) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&aggregate.context];
        let decl = context
            .aggregates
            .iter()
            .find(|a| a.name.name == aggregate.name);
        let command = decl.and_then(|a| a.commands.iter().find(|c| c.name.name == command));
        let parameters = command.and_then(|c| c.parameters.as_deref()).unwrap_or(&[]);
        self.lower_parameters(&aggregate.context, Some(&aggregate.name), parameters)
    }

    /// What a function gives back. A domain's body can fail with any error.
    pub fn function_result(&self, context: &str, function: &dn_syntax::ast::FunctionDecl) -> Type {
        match &function.return_type {
            Some(ty) => self.model.lower_type(context, None, ty),
            None if function.kind == dn_syntax::ast::FunctionKind::Domain => {
                Type::result(Type::Unit, Type::Errors(Vec::new()))
            }
            None => Type::Unit,
        }
    }

    /// The name of an aggregate's identity field.
    pub fn identity_field(&self, aggregate: &Resolved) -> String {
        let fields = self.model.fields(aggregate);
        fields
            .into_iter()
            .find(|(_, _, f)| f.is_identity)
            .map_or("id".to_string(), |(name, _, _)| name)
    }

    /// `void name(dn_int p_a, const Position *p_b, Result *out)`.
    pub fn signature(
        &self,
        name: &str,
        leading: &[String],
        parameters: &[(String, Type)],
        result: Option<&Type>,
    ) -> String {
        let mut list: Vec<String> = leading.to_vec();
        for (parameter, ty) in parameters {
            let c_type = self.c_type(ty);
            list.push(if self.is_scalar(ty) {
                format!("{c_type} p_{parameter}")
            } else {
                format!("const {c_type} *p_{parameter}")
            });
        }
        if let Some(result) = result {
            list.push(format!("{} *out", self.c_type(result)));
        }
        let list = if list.is_empty() {
            "void".to_string()
        } else {
            list.join(", ")
        };
        format!("void {name}({list})")
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// Makes each parameter's dn name refer to its C parameter.
    pub fn bind_parameters(&mut self, parameters: &[(String, Type)]) {
        for (name, ty) in parameters {
            let c = format!("p_{name}");
            self.parameter(&c);
            let value = if self.gen.is_scalar(ty) {
                c
            } else {
                format!("(*{c})")
            };
            self.bind(name, value, ty.clone());
        }
    }
}
