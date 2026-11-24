//! Working out constant expressions at compile time, for sizes and `#define`s.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::{Model, Program};
use dn_syntax::ast::{BinaryOp, Expr, ExprKind, Identifier, Path, UnaryOp};

/// The value of a whole-number expression made of literals and constants.
pub fn evaluate(program: &Program, model: &Model, context: &str, value: &Expr) -> Option<i64> {
    match &value.kind {
        ExprKind::Int(number) => Some(*number),
        ExprKind::Unary(UnaryOp::Negate, operand) => {
            Some(-evaluate(program, model, context, operand)?)
        }
        ExprKind::Binary(op, left, right) => {
            let (left, right) = (
                evaluate(program, model, context, left)?,
                evaluate(program, model, context, right)?,
            );
            match op {
                BinaryOp::Add => left.checked_add(right),
                BinaryOp::Subtract => left.checked_sub(right),
                BinaryOp::Multiply => left.checked_mul(right),
                BinaryOp::Divide => left.checked_div(right),
                BinaryOp::Remainder => left.checked_rem(right),
                _ => None,
            }
        }
        ExprKind::Name(name) => {
            let path = Path {
                segments: vec![Identifier {
                    name: name.clone(),
                    span: value.span,
                }],
            };
            let found = model
                .resolve(context, None, &path)
                .filter(|f| f.kind == SymbolKind::Const)?;
            let owner = &program.contexts[&found.context];
            let constant = owner.consts.iter().find(|c| c.name.name == found.name)?;
            evaluate(program, model, &found.context, &constant.value)
        }
        _ => None,
    }
}
