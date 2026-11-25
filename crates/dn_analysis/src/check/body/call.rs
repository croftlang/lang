//! Calls: functions, translations, services, constraints, enum variants and
//! built-in functions. Method calls are in `method`.

use dn_syntax::ast::{Argument, Expr, ExprId, ExprKind};
use dn_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::check::data::expr_path;
use crate::facts::Meaning;
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn call(
        &mut self,
        id: ExprId,
        callee: &Expr,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        if let ExprKind::Field(receiver, method) = &callee.kind {
            if self.as_type_path(callee).is_none() {
                let receiver = self.expr(receiver, None);
                self.note(Some(id), Meaning::Method);
                return self.method(&receiver, method, arguments, expected);
            }
        }
        let Some(path) = expr_path(callee) else {
            self.error("only a named function can be called", callee.span);
            return Type::Unknown;
        };
        let name = path.text();
        let Some(found) = self.checker.resolve(&self.scope, &path) else {
            if let Some(ty) = self.builtin_call(&name, arguments, span, expected) {
                self.note(Some(id), Meaning::Builtin(name.clone()));
                return ty;
            }
            if let Some((ty, variant)) = self.find_variant(&path, expected) {
                let parameters: Vec<(String, Type)> = (variant.fields.iter())
                    .map(|field| {
                        (
                            field.name.name.clone(),
                            self.checker.lower_type(&variant.scope, &field.ty),
                        )
                    })
                    .collect();
                self.check_arguments(arguments, &parameters, &name, span, false);
                if let Type::Named(owner) = &ty {
                    self.note(
                        Some(id),
                        Meaning::Variant(
                            owner.clone(),
                            variant.name.clone(),
                            variant.error.clone(),
                        ),
                    );
                }
                return ty;
            }
            self.error(format!("cannot find `{name}` here"), path.span());
            return Type::Unknown;
        };
        self.note(Some(id), Meaning::Declared(found.clone()));
        match found.kind {
            SymbolKind::Function | SymbolKind::Translation | SymbolKind::Constraint => {
                self.call_declared(&found, arguments, span)
            }
            SymbolKind::Service if matches!(self.kind, BodyKind::Service) => {
                self.call_declared(&found, arguments, span)
            }
            SymbolKind::Service => {
                self.error("only a service can run another service", span);
                Type::Unknown
            }
            SymbolKind::Value => match self.checker.single_value_inner(&Type::Named(found.clone()))
            {
                Some(inner) => {
                    let parameters = [("value".to_string(), inner)];
                    self.check_arguments(arguments, &parameters, &name, span, false);
                    Type::Named(found)
                }
                None => {
                    let message = format!("`{name}` is made with its fields: `{name} {{ ... }}`");
                    self.error(message, span);
                    Type::Unknown
                }
            },
            SymbolKind::Aggregate => {
                let message = format!(
                    "an aggregate is loaded by issuing a command: `issue Command on {name}(id)`"
                );
                self.error(message, span);
                Type::Unknown
            }
            kind => {
                self.error(
                    format!("`{name}` is {}, so it cannot be called", kind.describe()),
                    span,
                );
                Type::Unknown
            }
        }
    }

    /// A call to a function, translation, service or constraint declared in dn.
    pub(super) fn call_declared(
        &mut self,
        found: &Resolved,
        arguments: &[Argument],
        span: Span,
    ) -> Type {
        let scope = self.checker.scope_of(found);
        let (parameters, result) = if found.kind == SymbolKind::Constraint {
            let constraint = scope
                .context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name);
            (constraint.map(|c| c.parameters.as_slice()), Type::Bool)
        } else {
            let function = scope
                .context
                .functions
                .iter()
                .find(|f| f.name.name == found.name);
            let result = function.and_then(|f| f.return_type.as_ref());
            let result = result.map_or(Type::Unit, |ty| self.checker.lower_type(&scope, ty));
            (function.and_then(|f| f.parameters.as_deref()), result)
        };
        let parameters: Vec<(String, Type)> = (parameters.unwrap_or(&[]).iter())
            .map(|p| (p.name.name.clone(), self.checker.parameter_type(&scope, p)))
            .collect();
        self.check_arguments(arguments, &parameters, &found.name, span, false);
        result
    }
}
