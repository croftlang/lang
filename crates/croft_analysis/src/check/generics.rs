//! Generic templates in checking: types and calls that name an instance,
//! and asking for instances that don't exist yet.

use croft_syntax::ast::{FunctionDecl, Identifier, TypeExpr, ValueDecl};

use super::{Checker, Resolved, Scope};
use crate::generics::{instance_name, is_type_parameter_name, Request};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// The generic value a name refers to, if it is one.
    pub(super) fn generic_value(&self, found: &Resolved) -> Option<&'a ValueDecl> {
        if found.kind != SymbolKind::Value {
            return None;
        }

        let context = &self.program.contexts[&found.context];
        let value = context.values.iter().find(|v| v.name.name == found.name)?;

        (!value.type_parameters.is_empty()).then_some(value)
    }

    /// The generic function a name refers to, if it is one.
    pub(super) fn generic_function(&self, found: &Resolved) -> Option<&'a FunctionDecl> {
        let context = &self.program.contexts[&found.context];

        let function = context
            .functions
            .iter()
            .find(|f| f.name.name == found.name)?;

        (!function.type_parameters.is_empty()).then_some(function)
    }

    /// The instance of `template` for these type arguments, in the context
    /// using it, or `None` once it has been asked for, to be made in the next round.
    pub(super) fn instance(
        &self,
        scope: &Scope,
        template: &Resolved,
        arguments: Vec<TypeExpr>,
    ) -> Option<Resolved> {
        let context = &scope.context.name.name;
        let name = instance_name(&template.name, &arguments);

        if let Some(found) = self.lookup(context, &name) {
            return Some(found);
        }

        let request = Request {
            context: context.clone(),
            template_context: template.context.clone(),
            template: template.name.clone(),
            arguments,
        };

        let mut requests = self.requests.borrow_mut();

        if !requests.contains(&request) {
            requests.push(request);
        }

        None
    }

    /// A template's type parameters are single capital letters, each once.
    pub(super) fn check_type_parameters(&mut self, parameters: &[Identifier]) {
        for (index, parameter) in parameters.iter().enumerate() {
            if !is_type_parameter_name(&parameter.name) {
                let message = format!(
                    "a type parameter is a single capital letter, such as `T`, not `{}`",
                    parameter.name
                );

                self.error(message, parameter.span);
            }

            if parameters[..index].iter().any(|p| p.name == parameter.name) {
                let message = format!("type parameter `{}` is named twice", parameter.name);
                self.error(message, parameter.span);
            }
        }
    }

    /// Whether `ty` is an instance made from `template`.
    pub(super) fn is_instance_of(&self, ty: &Type, template: &Resolved) -> bool {
        matches!(ty, Type::Named(named) if named.name.starts_with(&format!("{}<", template.name)))
    }
}
