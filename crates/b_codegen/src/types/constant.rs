//! Working out constant expressions at compile time, for sizes and `#define`s.

use b_analysis::symbols::SymbolKind;
use b_analysis::{Model, Program};
use b_syntax::ast::{BinaryOp, Expr, ExprKind, Identifier, Path, UnaryOp};

/// The value of a whole-number expression made of literals and constants.
pub fn evaluate(program: &Program, model: &Model, context: &str, value: &Expr) -> Option<i64> {
    match &value.kind {
        ExprKind::Int(number) => Some(*number),
        ExprKind::Unary(UnaryOp::Negate, operand) => {
            Some(-evaluate(program, model, context, operand)?)
        }
        ExprKind::Unary(UnaryOp::BitNot, operand) => {
            Some(!evaluate(program, model, context, operand)?)
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
                BinaryOp::BitAnd => Some(left & right),
                BinaryOp::BitOr => Some(left | right),
                BinaryOp::BitXor => Some(left ^ right),
                BinaryOp::ShiftLeft => u32::try_from(right).ok().and_then(|r| left.checked_shl(r)),
                BinaryOp::ShiftRight => u32::try_from(right).ok().and_then(|r| left.checked_shr(r)),
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

/// What one whole unit is in a `Decimal` with this many places: 100 for two.
pub fn scale(places: u32) -> i64 {
    10_i64.pow(places)
}

/// A literal such as `2.5` as a whole number of the smallest unit: 250 for two places.
pub fn scaled(text: &str, places: u32) -> i64 {
    let (whole, part) = text.split_once('.').unwrap_or((text, ""));
    let mut digits: String = whole
        .chars()
        .chain(part.chars())
        .filter(|c| *c != '_')
        .collect();
    for _ in part.len()..places as usize {
        digits.push('0');
    }
    digits.parse().unwrap_or(0)
}

/// A `Range` bound for a `Decimal`: `0.5` or `10` as a whole number of its smallest unit.
pub fn decimal_bound(
    program: &Program,
    model: &Model,
    context: &str,
    value: &Expr,
    places: u32,
) -> Option<i64> {
    match &value.kind {
        ExprKind::Float(text) => Some(scaled(text, places)),
        ExprKind::Unary(UnaryOp::Negate, operand) => {
            Some(-decimal_bound(program, model, context, operand, places)?)
        }
        _ => evaluate(program, model, context, value)?.checked_mul(scale(places)),
    }
}
