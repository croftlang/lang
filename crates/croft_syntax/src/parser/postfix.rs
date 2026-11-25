//! Field access, calls, `?`, `with` and record literals.
//!
//! `.` and `?` carry on across a new line; the others must stay on the same line.

use super::{ParseResult, Parser};
use crate::ast::{Expr, ExprKind, Identifier, Path};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// Field access, calls, `?`, `with` and record literals after a primary.
    /// An `issue` target is read without `?`, so the `?` applies to the issue.
    pub(super) fn parse_postfix(&mut self, allow_try: bool) -> ParseResult<Expr> {
        let mut value = self.parse_primary()?;

        loop {
            let span = value.span;
            let same_line = !self.peek().newline_before;

            let kind = match self.peek().kind {
                TokenKind::Dot => {
                    self.advance();
                    ExprKind::Field(Box::new(value), self.expect_field_name()?)
                }
                TokenKind::LeftParen if same_line => {
                    ExprKind::Call(Box::new(value), self.parse_arguments()?)
                }
                TokenKind::Question if allow_try => {
                    self.advance();
                    ExprKind::Try(Box::new(value))
                }
                TokenKind::Keyword(Keyword::With) if same_line => {
                    self.advance();
                    ExprKind::With(Box::new(value), self.parse_field_values()?)
                }
                TokenKind::LeftBrace if same_line && self.allow_records => {
                    match record_path(&value) {
                        Some(path) => ExprKind::Record(path, self.parse_field_values()?),
                        None => return Ok(value),
                    }
                }
                _ => return Ok(value),
            };

            value = self.make_expr(kind, span);
        }
    }
}

/// The path a record literal names, when it ends in a type name: `Room.Enemy`.
fn record_path(value: &Expr) -> Option<Path> {
    let path = expr_to_path(value)?;

    let starts_upper = path
        .last()
        .name
        .starts_with(|c: char| c.is_ascii_uppercase());

    starts_upper.then_some(path)
}

pub(super) fn expr_to_path(value: &Expr) -> Option<Path> {
    match &value.kind {
        ExprKind::Name(name) => {
            let segment = Identifier {
                name: name.clone(),
                span: value.span,
            };

            Some(Path {
                segments: vec![segment],
            })
        }
        ExprKind::Field(inner, field) => {
            let mut path = expr_to_path(inner)?;
            path.segments.push(field.clone());
            Some(path)
        }
        _ => None,
    }
}
