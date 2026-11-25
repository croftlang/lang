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
    let range = constraints
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
        });

    match range {
        Some((low, high)) if low >= 0 && high <= 255 => "croft_u8",
        Some((low, high)) if low >= 0 && high <= 65535 => "croft_u16",
        Some((low, high)) if low >= -128 && high <= 127 => "croft_i8",
        Some((low, high)) if low >= -32768 && high <= 32767 => "croft_i16",
        _ => "croft_int",
    }
}
