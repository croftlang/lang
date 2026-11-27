//! The built-in methods of options, and the lambdas list and option methods take.

use croft_syntax::ast::{Argument, Expr, ExprKind};

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a built-in method on an `Option` of `item`, or `None` if
    /// `name` isn't one.
    pub(super) fn option_method(
        &mut self,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        span: croft_syntax::Span,
        expected: Option<&Type>,
    ) -> Option<Type> {
        match name {
            "map" => {
                let mapped = self.lambda_argument(name, arguments, item, span);
                Some(Type::option(mapped))
            }
            "unwrap_or" => {
                let item = if item.is_unknown() {
                    expected.cloned().unwrap_or(Type::Unknown)
                } else {
                    item.clone()
                };

                self.fixed(
                    name,
                    arguments,
                    std::slice::from_ref(&item),
                    item.clone(),
                    span,
                )
            }
            _ => None,
        }
    }

    /// The single lambda argument of a list or option method, whose parameter is an `item`.
    pub(super) fn lambda_argument(
        &mut self,
        name: &str,
        arguments: &[Argument],
        item: &Type,
        span: croft_syntax::Span,
    ) -> Type {
        let [argument] = arguments else {
            self.error(
                format!("`{name}` takes 1 argument, a lambda such as `e -> e.id`"),
                span,
            );

            return Type::Unknown;
        };

        let example = "e -> e.id";
        self.lambda_with(name, &argument.value, std::slice::from_ref(item), example)
    }

    /// The type a lambda passed to `name` gives back, when its parameters
    /// are given `parameters` in order. `example` is a lambda that fits, for
    /// the message when the value isn't a lambda or has another number of
    /// parameters.
    pub(super) fn lambda_with(
        &mut self,
        name: &str,
        lambda: &Expr,
        parameters: &[Type],
        example: &str,
    ) -> Type {
        let ExprKind::Lambda(written, body) = &lambda.kind else {
            self.error(
                format!("`{name}` needs a lambda, such as `{example}`"),
                lambda.span,
            );

            return Type::Unknown;
        };

        if written.len() != parameters.len() {
            let count = match parameters.len() {
                1 => "1 parameter".to_string(),
                count => format!("{count} parameters"),
            };

            let message = format!(
                "the lambda passed to `{name}` takes {count}, such as `{example}`, found {}",
                written.len()
            );

            self.error(message, lambda.span);
            return Type::Unknown;
        }

        self.nested(|checker| {
            for (parameter, ty) in written.iter().zip(parameters) {
                checker.bind(&parameter.name, ty.clone(), false, Some(parameter.span));
            }

            checker.expr(body, None)
        })
    }
}
