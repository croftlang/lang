//! How many bytes a whole number needs, from its `Range` constraint.

use b_analysis::{Model, Program};
use b_syntax::ast::ConstraintUse;

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
        Some((low, high)) if low >= 0 && high <= 255 => "b_u8",
        Some((low, high)) if low >= 0 && high <= 65535 => "b_u16",
        Some((low, high)) if low >= -128 && high <= 127 => "b_i8",
        Some((low, high)) if low >= -32768 && high <= 32767 => "b_i16",
        _ => "b_int",
    }
}
