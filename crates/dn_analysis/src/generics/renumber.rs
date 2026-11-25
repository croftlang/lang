//! Fresh ids for the expressions of an instance, so facts about them are
//! kept apart from the template's and every other instance's.

use dn_syntax::ast::{Expr, ExprKind, Statement};
use dn_syntax::SourceId;

/// Gives every expression in `value` a new id from `source`, and writes the
/// type arguments into the types of its `let`s.
pub fn renumber(
    value: &mut Expr,
    source: SourceId,
    next: &mut u32,
    types: &dyn Fn(&mut dn_syntax::ast::TypeExpr),
) {
    value.id.source = source;
    value.id.index = *next;
    *next += 1;
    let mut each = |child: &mut Expr| renumber(child, source, next, types);
    match &mut value.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Name(_)
        | ExprKind::Break => {}
        ExprKind::Map(entries) => {
            for (key, entry) in entries {
                each(key);
                each(entry);
            }
        }
        ExprKind::Field(inner, _)
        | ExprKind::Unary(_, inner)
        | ExprKind::Try(inner)
        | ExprKind::Lambda(_, inner) => each(inner),
        ExprKind::Call(callee, arguments) => {
            each(callee);
            for argument in arguments {
                each(&mut argument.value);
            }
        }
        ExprKind::Record(_, fields) | ExprKind::Emit(_, fields) => {
            for field in fields {
                if let Some(value) = &mut field.value {
                    each(value);
                }
            }
        }
        ExprKind::With(base, fields) => {
            each(base);
            for field in fields {
                if let Some(value) = &mut field.value {
                    each(value);
                }
            }
        }
        ExprKind::List(items) => items.iter_mut().for_each(each),
        ExprKind::Binary(_, left, right)
        | ExprKind::OrElse(left, right)
        | ExprKind::If(left, right) => {
            each(left);
            each(right);
        }
        ExprKind::Match(subject, arms) => {
            each(subject);
            for arm in arms {
                each(&mut arm.value);
            }
        }
        ExprKind::Loop(_, body) => each(body),
        ExprKind::Block(statements) => {
            for statement in statements {
                match statement {
                    Statement::Let(_, ty, value) => {
                        if let Some(ty) = ty {
                            types(ty);
                        }
                        each(value);
                    }
                    Statement::Set(_, value) | Statement::Expr(value) => each(value),
                }
            }
        }
        ExprKind::Issue(issue) => {
            each(&mut issue.target);
            for argument in &mut issue.arguments {
                each(&mut argument.value);
            }
        }
        ExprKind::Native(_, arguments) => {
            for argument in arguments {
                each(&mut argument.value);
            }
        }
    }
}
