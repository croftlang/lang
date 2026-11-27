//! The built-in methods of results.

use croft_syntax::ast::Argument;
use croft_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `map` changes the value of a success and `map_err` the error of a
    /// failure, each with a lambda; `unwrap_or` gives the value, or the
    /// fallback when it failed.
    pub(super) fn result_method(
        &mut self,
        (value, error): (&Type, &Type),
        name: &str,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Option<Type> {
        match name {
            "map" => {
                let mapped = self.lambda_argument(name, arguments, value, span);
                Some(Type::result(mapped, error.clone()))
            }
            "map_err" => {
                let mapped = self.lambda_argument(name, arguments, error, span);

                if !self.is_error_or_unknown(&mapped) {
                    self.error(
                        format!("the lambda passed to `map_err` must give back an error, found `{mapped}`"),
                        span,
                    );

                    return Some(Type::result(value.clone(), Type::Unknown));
                }

                Some(Type::result(value.clone(), mapped))
            }
            "unwrap_or" => {
                let value = if value.is_unknown() {
                    expected.cloned().unwrap_or(Type::Unknown)
                } else {
                    value.clone()
                };

                self.fixed(
                    name,
                    arguments,
                    std::slice::from_ref(&value),
                    value.clone(),
                    span,
                )
            }
            _ => None,
        }
    }

    /// Whether a type is an error type, or not known.
    fn is_error_or_unknown(&self, ty: &Type) -> bool {
        match ty {
            Type::Unknown | Type::Errors(_) => true,
            Type::Named(named) => self.checker.is_error_type(named),
            _ => false,
        }
    }
}
