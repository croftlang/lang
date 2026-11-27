//! The built-in methods of strings.

use croft_syntax::ast::Argument;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a built-in string method, or `None` if `name` isn't one.
    pub(super) fn string_method(
        &mut self,
        name: &str,
        arguments: &[Argument],
        span: croft_syntax::Span,
        expected: Option<&Type>,
    ) -> Option<Type> {
        let s = Type::String;

        match name {
            "contains" | "starts_with" | "ends_with" => {
                self.fixed(name, arguments, &[s], Type::Bool, span)
            }
            "to_upper" | "to_lower" | "trim" => self.fixed(name, arguments, &[], s, span),
            "slice" => self.fixed(name, arguments, &[Type::Int, Type::Int], s, span),
            "split" => self.fixed(name, arguments, &[s], Type::list(Type::String), span),
            "to_int" => self.fixed(name, arguments, &[], Type::option(Type::Int), span),
            "code_at" => self.fixed(name, arguments, &[Type::Int], Type::option(Type::Int), span),
            "to_decimal" => {
                let decimal = self.decimal_places_expected(expected, span);
                self.fixed(name, arguments, &[], Type::option(decimal), span)
            }
            "to_bytes" => self.fixed(name, arguments, &[], Type::Bytes, span),
            "index_of" => self.fixed(name, arguments, &[s], Type::option(Type::Int), span),
            "replace" => self.fixed(name, arguments, &[s.clone(), s.clone()], s, span),
            "lines" => self.fixed(name, arguments, &[], Type::list(Type::String), span),
            "repeat" => self.fixed(name, arguments, &[Type::Int], s, span),
            "percent_encode" => self.fixed(name, arguments, &[], s, span),
            "percent_decode" => self.fixed(name, arguments, &[], Type::option(s), span),
            "pad_left" | "pad_right" => {
                self.fixed(name, arguments, &[Type::Int, s.clone()], s, span)
            }
            _ => None,
        }
    }

    /// The `Decimal` that `text.to_decimal()` reads, taken from where its
    /// answer goes, since the text alone doesn't say how many places it
    /// has. Anywhere that doesn't expect an `Option<Decimal<N>>` is an error.
    fn decimal_places_expected(
        &mut self,
        expected: Option<&Type>,
        span: croft_syntax::Span,
    ) -> Type {
        if let Some(Type::Option(item)) = expected {
            if let Type::Decimal(_) = **item {
                return (**item).clone();
            }
        }

        let message = "`to_decimal` gives an `Option<Decimal<N>>`, so it needs to go where one \
             with known places is expected, such as \
             `let price: Option<Decimal<2>> = text.to_decimal()`";
        self.error(message, span);

        Type::Unknown
    }
}
