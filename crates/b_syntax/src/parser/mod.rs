//! Turns tokens into a syntax tree.

mod aggregate;
mod arguments;
mod behaviour;
mod block;
mod boundary;
mod constraints;
mod contract;
mod data;
mod domain;
mod expression;
mod file;
mod postfix;
mod primary;
mod repeat;
#[cfg(test)]
mod tests;
mod tokens;
mod types;

use crate::ast::{Expr, ExprId, ExprKind, File};
use crate::lexer::tokenize;
use crate::span::{Diagnostic, SourceId};
use crate::token::Token;

pub fn parse(source: &str, source_id: SourceId) -> Result<File, Diagnostic> {
    let tokens = tokenize(source, source_id)?;
    Parser {
        tokens,
        position: 0,
        allow_records: true,
        source_id,
        next_expr: 0,
    }
    .parse_file()
}

type ParseResult<T> = Result<T, Diagnostic>;

pub(crate) struct Parser {
    tokens: Vec<Token>,
    position: usize,
    /// False inside an `if` or `match` condition, where `{` starts the block.
    allow_records: bool,
    source_id: SourceId,
    next_expr: u32,
}

impl Parser {
    fn make_expr(&mut self, kind: ExprKind, span: crate::span::Span) -> Expr {
        let id = ExprId {
            source: self.source_id,
            index: self.next_expr,
        };
        self.next_expr += 1;
        Expr { kind, span, id }
    }
}
