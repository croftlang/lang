//! The built-in methods of strings.

use croft_syntax::ast::Argument;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn string_method(
        &mut self,
        name: &str,
        arguments: &[Argument],
        span: croft_syntax::Span,
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
            "to_bytes" => self.fixed(name, arguments, &[], Type::Bytes, span),
            "index_of" => self.fixed(name, arguments, &[s], Type::option(Type::Int), span),
            "replace" => self.fixed(name, arguments, &[s.clone(), s.clone()], s, span),
            "lines" => self.fixed(name, arguments, &[], Type::list(Type::String), span),
            "repeat" => self.fixed(name, arguments, &[Type::Int], s, span),
            "pad_left" | "pad_right" => {
                self.fixed(name, arguments, &[Type::Int, s.clone()], s, span)
            }
            _ => None,
        }
    }
}
