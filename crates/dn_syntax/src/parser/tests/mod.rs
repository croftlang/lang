mod declarations;
mod examples;
mod expressions;

use crate::ast::{ContextDecl, ContextItem, Expr, File, Statement, TopItem};
use crate::span::SourceId;

fn parse_ok(source: &str) -> File {
    match super::parse(source, SourceId(0)) {
        Ok(file) => file,
        Err(error) => panic!(
            "{}:{}: {}",
            error.span.line, error.span.column, error.message
        ),
    }
}

fn parse_error(source: &str) -> String {
    super::parse(source, SourceId(0))
        .expect_err("expected a parse error")
        .message
}

fn only_context(file: &File) -> &ContextDecl {
    match file.items.as_slice() {
        [TopItem::Context(context)] => context,
        other => panic!("expected one context, found {other:?}"),
    }
}

/// The body of the only function in `context Test { ... }`.
fn function_body(source: &str) -> Expr {
    let file = parse_ok(&format!("context Test {{\n{source}\n}}"));
    match &only_context(&file).items[..] {
        [ContextItem::Function(function)] => function.body.clone().expect("a body"),
        other => panic!("expected one function, found {other:?}"),
    }
}

/// The statements of `fn test() = { ... }`.
fn statements(body: &str) -> Vec<Statement> {
    match function_body(&format!("fn test() = {{\n{body}\n}}")).kind {
        crate::ast::ExprKind::Block(statements) => statements,
        other => panic!("expected a block, found {other:?}"),
    }
}
