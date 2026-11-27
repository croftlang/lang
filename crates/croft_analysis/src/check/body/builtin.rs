//! Built-in functions: the numbers of `Std.Numbers`, `from_code` and
//! `from_hex` of `Std.Text`, the decoders of `Std.Encoding`, `Some`, `Ok`,
//! `Err`, and the built-in constraints `Range` and `Length` called directly.

use croft_syntax::ast::Argument;
use croft_syntax::Span;

use super::BodyChecker;
use crate::check::known::Known;
use crate::check::publish::imported_standard;
use crate::standard::{ENCODING_CONTEXT, SHARED_FUNCTIONS, TEXT_CONTEXT};
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
        let from = self.check_function_import(name, span, expected);

        let ty = match name {
            "max" | "min" => {
                let first = self.one_argument(name, arguments, 0, 2, span, None);

                if !first.is_number() {
                    self.error(format!("`{name}` needs numbers, found `{first}`"), span);
                }

                self.one_argument(name, arguments, 1, 2, span, Some(&first));

                first
            }
            "clamp" => {
                let first = self.one_argument(name, arguments, 0, 3, span, None);

                if !first.is_number() {
                    self.error(format!("`{name}` needs numbers, found `{first}`"), span);
                }

                for index in 1..3 {
                    self.one_argument(name, arguments, index, 3, span, Some(&first));
                }

                first
            }
            "sign" => {
                let number = self.one_argument(name, arguments, 0, 1, span, None);

                if !number.is_number() {
                    self.error(format!("`{name}` needs a number, found `{number}`"), span);
                }

                Type::Int
            }
            "abs" => self.one_argument(name, arguments, 0, 1, span, Some(&Type::Int)),
            "pow" | "checked_add" | "checked_subtract" | "checked_multiply" => {
                for index in 0..2 {
                    self.one_argument(name, arguments, index, 2, span, Some(&Type::Int));
                }

                match name {
                    "pow" => Type::Int,
                    _ => Type::option(Type::Int),
                }
            }
            "from_code" => {
                self.one_argument(name, arguments, 0, 1, span, Some(&Type::Int));
                self.check_known_code(arguments);

                Type::String
            }
            "from_hex" | "from_base64"
                if name == "from_base64" || from == Some(ENCODING_CONTEXT) =>
            {
                self.one_argument(name, arguments, 0, 1, span, Some(&Type::String));

                Type::option(Type::Bytes)
            }
            "from_hex" => {
                self.one_argument(name, arguments, 0, 1, span, Some(&Type::String));

                // With no import to say which it is, nothing more is reported.
                match from {
                    Some(_) => Type::option(Type::Int),
                    None => Type::option(Type::Unknown),
                }
            }
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

    /// Checks that the context may call the built-in function `name`, and
    /// gives the standard context it comes from. A function two standard
    /// contexts provide, such as `from_hex`, means what the context imports;
    /// with no import, the type `expected` here picks the one to name, or
    /// both are named when it doesn't.
    fn check_function_import(
        &mut self,
        name: &str,
        span: Span,
        expected: Option<&Type>,
    ) -> Option<&'static str> {
        let context = self.scope.context;
        let from = imported_standard(context, name);

        if from.is_some() || !SHARED_FUNCTIONS.contains(&name) {
            self.checker.check_standard(context, name, span);
            return from;
        }

        let wanted = match expected {
            Some(Type::Option(item)) if **item == Type::Bytes => Some(ENCODING_CONTEXT),
            Some(Type::Option(item)) if **item == Type::Int => Some(TEXT_CONTEXT),
            _ => None,
        };

        self.checker
            .report_missing_shared_import(name, wanted, span);

        wanted
    }

    /// Reports a character code given to `from_code` that is known when the
    /// program is compiled and isn't from 0 to 255.
    fn check_known_code(&mut self, arguments: &[Argument]) {
        let Some(argument) = arguments.first() else {
            return;
        };

        let Some(Known::Number { units, places: 0 }) = self.known_local_value(&argument.value)
        else {
            return;
        };

        if (0..=255).contains(&units) {
            return;
        }

        let message = format!("a character code is from 0 to 255, found {units}");
        self.error(message, argument.value.span);
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
