//! Whether a name is read anywhere in some code, so a `let` that is never
//! read needs no C variable.

use croft_syntax::ast::{Expr, ExprKind, Statement};

/// Whether `name` is read in any of these statements.
pub fn statements_use(statements: &[Statement], name: &str) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Let(_, _, value) => uses(value, name),
        Statement::Set(target, value) => target.name == name || uses(value, name),
        Statement::Expr(value) => uses(value, name),
    })
}

/// Whether `name` is read anywhere in `value`. Shorthand fields, such as
/// `{ enemy_id }`, read the name too.
pub fn uses(value: &Expr, name: &str) -> bool {
    match &value.kind {
        ExprKind::Name(found) => found == name,
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Break => false,
        ExprKind::Field(inner, _) | ExprKind::Unary(_, inner) | ExprKind::Try(inner) => {
            uses(inner, name)
        }
        ExprKind::Call(callee, arguments) => {
            uses(callee, name) || any_uses(arguments.iter().map(|a| &a.value), name)
        }
        ExprKind::Native(_, arguments) => any_uses(arguments.iter().map(|a| &a.value), name),
        ExprKind::Record(_, fields) | ExprKind::Emit(_, fields) => {
            fields.iter().any(|f| match &f.value {
                Some(value) => uses(value, name),
                None => f.name.name == name,
            })
        }
        ExprKind::With(base, fields) => {
            uses(base, name)
                || fields.iter().any(|f| {
                    f.value
                        .as_ref()
                        .map_or(f.name.name == name, |v| uses(v, name))
                })
        }
        ExprKind::List(items) => any_uses(items.iter(), name),
        ExprKind::Map(entries) => entries.iter().any(|(k, v)| uses(k, name) || uses(v, name)),
        ExprKind::Binary(_, left, right)
        | ExprKind::OrElse(left, right)
        | ExprKind::If(left, right) => uses(left, name) || uses(right, name),
        ExprKind::Lambda(parameters, body) => {
            !parameters.iter().any(|p| p.name == name) && uses(body, name)
        }
        ExprKind::Match(subject, arms) => {
            uses(subject, name) || arms.iter().any(|arm| uses(&arm.value, name))
        }
        ExprKind::Block(statements) => statements_use(statements, name),
        ExprKind::Issue(issue) => {
            uses(&issue.target, name) || any_uses(issue.arguments.iter().map(|a| &a.value), name)
        }
        ExprKind::Loop(_, body) => uses(body, name),
    }
}

fn any_uses<'e>(values: impl IntoIterator<Item = &'e Expr>, name: &str) -> bool {
    values.into_iter().any(|value| uses(value, name))
}
