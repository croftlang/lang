//! Working out constant expressions at compile time, for sizes and `#define`s.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::{Model, Program};
use croft_syntax::ast::{BinaryOp, Expr, ExprKind, Identifier, Path, UnaryOp};

/// The value of a whole-number expression made of literals and constants.
pub fn evaluate(program: &Program, model: &Model, context: &str, value: &Expr) -> Option<i64> {
    match &value.kind {
        ExprKind::Int(number) => Some(*number),
        ExprKind::Unary(UnaryOp::Negate, operand) => {
            whole(-evaluate(program, model, context, operand)?)
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
                BinaryOp::Add => whole(left.checked_add(right)?),
                BinaryOp::Subtract => whole(left.checked_sub(right)?),
                BinaryOp::Multiply => whole(left.checked_mul(right)?),
                BinaryOp::Divide => whole(left.checked_div(right)?),
                BinaryOp::Remainder => left.checked_rem(right),
                BinaryOp::BitAnd => Some(left & right),
                BinaryOp::BitOr => Some(left | right),
                BinaryOp::BitXor => Some(left ^ right),
                BinaryOp::ShiftLeft => shifted(left, right, i32::wrapping_shl),
                BinaryOp::ShiftRight => shifted(left, right, i32::wrapping_shr),
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

/// `number` when it fits the 32 bits of an `Int`. Arithmetic that goes past
/// them is an error `croft check` reports, so it has no value here.
fn whole(number: i64) -> Option<i64> {
    i32::try_from(number).ok().map(i64::from)
}

/// `value` shifted by `amount` as the 32 bits of an `Int` are, through
/// `shift`; `None` for an amount outside 0 to 31.
fn shifted(value: i64, amount: i64, shift: fn(i32, u32) -> i32) -> Option<i64> {
    let amount = u32::try_from(amount).ok().filter(|amount| *amount < 32)?;

    Some(i64::from(shift(i32::try_from(value).ok()?, amount)))
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
