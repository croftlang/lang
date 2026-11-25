//! The built-in methods of options, and the lambdas list and option methods take.

use croft_syntax::ast::{Argument, ExprKind};

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
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

    /// The single lambda argument of a list or option method, called with `item`.
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
        let ExprKind::Lambda(parameters, body) = &argument.value.kind else {
            self.error(
                format!("`{name}` needs a lambda, such as `e -> e.id`"),
                argument.value.span,
            );
            return Type::Unknown;
        };
        self.nested(|checker| {
            for parameter in parameters {
                checker.bind(&parameter.name, item.clone(), false, Some(parameter.span));
            }
            checker.expr(body, None)
        })
    }
}
