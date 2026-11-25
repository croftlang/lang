//! Calls to generic functions: their type parameters worked out from the
//! arguments, and the instance for them called like any function.

use std::collections::HashMap;

use dn_syntax::ast::{Argument, ExprId, Identifier, TypeExpr};
use dn_syntax::Span;

use super::BodyChecker;
use crate::facts::Meaning;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn call_generic(
        &mut self,
        id: ExprId,
        template: &Resolved,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        let Some(declaration) = self.checker.generic_function(template) else {
            return Type::Unknown;
        };
        let parameters = declaration.parameters.as_deref().unwrap_or(&[]);
        let template_scope = self.checker.scope_of(template);
        let mut bound = HashMap::new();
        // What the call must give back says the most, as with
        // `let ids: List<RoomId> = both(2, 5)`, so it is matched first.
        if let (Some(written), Some(expected)) = (&declaration.return_type, expected) {
            let parameters = &declaration.type_parameters;
            self.checker
                .match_type(&template_scope, written, expected, parameters, &mut bound);
        }
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = match &argument.name {
                Some(name) => parameters.iter().find(|p| p.name.name == name.name),
                None => parameters.get(index),
            };
            let found = self.expr(&argument.value, None);
            if let Some(written) = parameter.and_then(|p| p.types.first()) {
                let parameters = &declaration.type_parameters;
                self.checker
                    .match_type(&template_scope, written, &found, parameters, &mut bound);
            }
        }
        let Some(arguments_types) =
            self.type_arguments(&declaration.type_parameters, &bound, &template.name, span)
        else {
            return Type::Unknown;
        };
        let Some(instance) = self
            .checker
            .instance(&self.scope, template, arguments_types)
        else {
            return Type::Unknown;
        };
        self.note(Some(id), Meaning::Declared(instance.clone()));
        self.call_declared(&instance, arguments, span)
    }

    /// The type arguments for a template's parameters, written out, once
    /// each has been worked out. Reports the first that hasn't.
    pub(super) fn type_arguments(
        &mut self,
        parameters: &[Identifier],
        bound: &HashMap<String, Type>,
        template: &str,
        span: Span,
    ) -> Option<Vec<TypeExpr>> {
        let context = self.scope.context.name.name.clone();
        let mut arguments = Vec::new();
        for parameter in parameters {
            let written = bound
                .get(&parameter.name)
                .and_then(|ty| self.checker.type_expr_of(ty, &context));
            match written {
                Some(written) => arguments.push(written),
                None => {
                    let message = format!(
                        "cannot work out `{}` for `{template}` from what it is given",
                        parameter.name
                    );
                    self.error(message, span);
                    return None;
                }
            }
        }
        Some(arguments)
    }
}
