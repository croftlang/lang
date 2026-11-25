//! Built-in functions: `max`, `min`, `abs`, `Some`, `Ok`, `Err`, and the
//! built-in constraints `Range` and `Length` called directly.

use croft_syntax::ast::Argument;
use croft_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a built-in function call, or `None` if `name` isn't one.
    pub(super) fn builtin_call(
        &mut self,
        name: &str,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Option<Type> {
        let context = self.scope.context;
        self.checker.check_standard(context, name, span);
        let ty = match name {
            "max" | "min" => {
                let first = self.one_argument(name, arguments, 0, 2, span, None);
                if !first.is_number() {
                    self.error(format!("`{name}` needs numbers, found `{first}`"), span);
                }
                self.one_argument(name, arguments, 1, 2, span, Some(&first));
                first
            }
            "abs" => self.one_argument(name, arguments, 0, 1, span, Some(&Type::Int)),
            "Some" => {
                let item = match expected {
                    Some(Type::Option(item)) => Some((**item).clone()),
                    _ => None,
                };
                Type::option(self.one_argument(name, arguments, 0, 1, span, item.as_ref()))
            }
            "Ok" | "Err" => {
                let (value, error) = match expected {
                    Some(Type::Result(value, error)) => ((**value).clone(), (**error).clone()),
                    _ => (Type::Unknown, Type::Unknown),
                };
                if name == "Ok" {
                    Type::result(
                        self.one_argument(name, arguments, 0, 1, span, Some(&value)),
                        error,
                    )
                } else {
                    Type::result(
                        value,
                        self.one_argument(name, arguments, 0, 1, span, Some(&error)),
                    )
                }
            }
            "Range" => {
                for index in 0..3 {
                    self.one_argument(name, arguments, index, 3, span, Some(&Type::Int));
                }
                Type::Bool
            }
            "Length" => {
                let items =
                    self.one_argument(name, arguments, 0, arguments.len().clamp(2, 3), span, None);
                if !matches!(items, Type::String | Type::List(_) | Type::Unknown) {
                    self.error(
                        format!("`Length` needs a string or list, found `{items}`"),
                        span,
                    );
                }
                for index in 1..arguments.len().clamp(2, 3) {
                    self.one_argument(name, arguments, index, 3, span, Some(&Type::Int));
                }
                Type::Bool
            }
            _ => return None,
        };
        Some(ty)
    }

    /// Checks argument `index` of `count`, reporting a wrong count once.
    pub(super) fn one_argument(
        &mut self,
        name: &str,
        arguments: &[Argument],
        index: usize,
        count: usize,
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        if index == 0 && arguments.len() != count {
            self.error(
                format!(
                    "`{name}` takes {count} arguments, found {}",
                    arguments.len()
                ),
                span,
            );
        }
        let Some(argument) = arguments.get(index) else {
            return expected.cloned().unwrap_or(Type::Unknown);
        };
        match expected {
            Some(expected) => self.expect(&argument.value, expected),
            None => self.expr(&argument.value, None),
        }
    }
}
