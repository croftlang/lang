//! `Bytes`: literals written as a list of numbers, and its methods.

use dn_syntax::ast::{Argument, Expr, ExprKind};
use dn_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `[0x44, 0x4E]` where `Bytes` is expected: each item a number from 0 to 255.
    pub(super) fn bytes_literal(&mut self, items: &[Expr]) -> Type {
        for item in items {
            self.expect(item, &Type::Int);
            if let ExprKind::Int(number) = item.kind {
                if !(0..=255).contains(&number) {
                    self.error(
                        format!("a byte is from 0 to 255, found {number}"),
                        item.span,
                    );
                }
            }
        }
        Type::Bytes
    }

    /// `get` gives the byte at a position, `add` a copy with one more byte on
    /// the end, and `slice` the bytes from one position up to another.
    /// `to_text` is imported from `Std.Text`.
    pub(super) fn bytes_method(
        &mut self,
        name: &str,
        arguments: &[Argument],
        span: Span,
    ) -> Option<Type> {
        let int = Type::Int;
        match name {
            "get" => self.fixed(name, arguments, &[int], Type::option(Type::Int), span),
            "add" => self.fixed(name, arguments, &[int], Type::Bytes, span),
            "slice" => self.fixed(name, arguments, &[int.clone(), int], Type::Bytes, span),
            "to_text" => self.fixed(name, arguments, &[], Type::String, span),
            _ => None,
        }
    }
}
