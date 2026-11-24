//! Prints dn expressions back as dn source, for messages and doc comments.

use dn_syntax::ast::{Expr, ExprKind, UnaryOp};

pub fn expression_text(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Int(value) => value.to_string(),
        ExprKind::Float(value) => format!("{value:?}"),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::String(value) => format!("{value:?}"),
        ExprKind::Name(name) => name.clone(),
        ExprKind::Field(base, field) => format!("{}.{}", expression_text(base), field.name),
        ExprKind::Call(name, arguments) => {
            let arguments: Vec<String> = arguments.iter().map(expression_text).collect();
            format!("{}({})", name.name, arguments.join(", "))
        }
        ExprKind::Unary(op, operand) => {
            let symbol = match op {
                UnaryOp::Not => "!",
                UnaryOp::Negate => "-",
            };
            format!("{symbol}{}", wrapped(operand))
        }
        ExprKind::Binary(op, left, right) => {
            format!("{} {} {}", wrapped(left), op.symbol(), wrapped(right))
        }
    }
}

fn wrapped(expr: &Expr) -> String {
    match expr.kind {
        ExprKind::Binary(..) => format!("({})", expression_text(expr)),
        _ => expression_text(expr),
    }
}
