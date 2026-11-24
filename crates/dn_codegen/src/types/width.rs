//! How many bytes a whole number needs, from its `Range` rule.

use dn_analysis::{Model, Program};
use dn_syntax::ast::Rule;

use super::constant::evaluate;

/// The smallest C integer type that holds every value a `Range` allows.
pub fn int_width(program: &Program, model: &Model, context: &str, rules: &[Rule]) -> &'static str {
    let range = rules
        .iter()
        .filter(|r| !r.each && r.constraint.name.text() == "Range")
        .find_map(|rule| {
            let [low, high] = rule.constraint.arguments.as_slice() else {
                return None;
            };
            Some((
                evaluate(program, model, context, low)?,
                evaluate(program, model, context, high)?,
            ))
        });
    match range {
        Some((low, high)) if low >= 0 && high <= 255 => "dn_u8",
        Some((low, high)) if low >= 0 && high <= 65535 => "dn_u16",
        Some((low, high)) if low >= -128 && high <= 127 => "dn_i8",
        Some((low, high)) if low >= -32768 && high <= 32767 => "dn_i16",
        _ => "dn_int",
    }
}
