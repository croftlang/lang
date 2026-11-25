//! Reading b source code: tokens, syntax tree and parser.

pub mod ast;
pub mod keyword;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use parser::parse;
pub use span::{Diagnostic, SourceId, Span};
