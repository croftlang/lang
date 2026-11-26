//! How many bytes a whole number needs, from its `Range` constraint.

use croft_analysis::{Model, Program};
use croft_syntax::ast::ConstraintUse;

use super::constant::evaluate;

/// The smallest C integer type that holds every value a `Range` allows.
pub fn int_width(
    program: &Program,
    model: &Model,
    context: &str,
    constraints: &[ConstraintUse],
) -> &'static str {
    match declared_range(program, model, context, constraints) {
        Some((low, high)) => width_for(low, high),
        None => "croft_int",
    }
}

/// The lowest and highest values a `Range` among `constraints` allows, when
/// both are constants.
pub fn declared_range(
    program: &Program,
    model: &Model,
    context: &str,
    constraints: &[ConstraintUse],
) -> Option<(i64, i64)> {
    constraints
        .iter()
        .filter(|r| !r.each && r.name.text() == "Range")
        .find_map(|constraint| {
            let [low, high] = constraint.arguments.as_slice() else {
                return None;
            };

            Some((
                evaluate(program, model, context, low)?,
                evaluate(program, model, context, high)?,
            ))
        })
}

/// The smallest C integer type that holds every value from `low` to `high`.
/// A signed type comes before `croft_u16`, so that every narrow number that
/// fits a C `int` is promoted to a signed `int` in arithmetic, even where
/// `int` has only 16 bits.
pub fn width_for(low: i64, high: i64) -> &'static str {
    match (low, high) {
        (low, high) if low >= 0 && high <= 255 => "croft_u8",
        (low, high) if low >= -128 && high <= 127 => "croft_i8",
        (low, high) if low >= -32768 && high <= 32767 => "croft_i16",
        (low, high) if low >= 0 && high <= 65535 => "croft_u16",
        _ => "croft_int",
    }
}
