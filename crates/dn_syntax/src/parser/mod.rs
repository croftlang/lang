//! Turns tokens into a syntax tree.

mod aggregate;
mod application;
mod arguments;
mod behaviour;
mod block;
mod boundary;
mod data;
mod expression;
mod file;
mod postfix;
mod primary;
mod rules;
#[cfg(test)]
mod tests;
mod tokens;
mod types;

use crate::ast::File;
use crate::lexer::tokenize;
use crate::span::{Diagnostic, SourceId};
use crate::token::Token;

pub fn parse(source: &str, source_id: SourceId) -> Result<File, Diagnostic> {
    let tokens = tokenize(source, source_id)?;
    Parser {
        tokens,
        position: 0,
        allow_records: true,
    }
    .parse_file()
}

type ParseResult<T> = Result<T, Diagnostic>;

pub(crate) struct Parser {
    tokens: Vec<Token>,
    position: usize,
    /// False inside an `if` or `match` condition, where `{` starts the block.
    allow_records: bool,
}
