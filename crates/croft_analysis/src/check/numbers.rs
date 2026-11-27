//! The functions of `Std.Numbers` worked out on known values: `max`, `min`,
//! `abs`, `clamp`, `pow`, `sign` and the checked arithmetic, so what they give
//! is checked against constraints when the program is compiled, and one with
//! no answer in 32 bits is an error found by `croft check`.

use croft_syntax::ast::BinaryOp;

use super::known::{fits, Fault, Known, Outcome, INT_LOW};

/// What calling the `Std.Numbers` function `name` with these known
/// arguments gives, or `None` when it isn't one, an argument doesn't suit
/// it, or it has no answer.
pub(crate) fn number_function(name: &str, arguments: &[Known]) -> Option<Known> {
    if number_fault(name, arguments).is_some() {
        return None;
    }

    match (name, arguments) {
        ("max", [a, b]) => Some(if is_true(BinaryOp::Less, a, b)? { b } else { a }.clone()),
        ("min", [a, b]) => Some(
            if is_true(BinaryOp::Greater, a, b)? {
                b
            } else {
                a
            }
            .clone(),
        ),
        ("abs", [a]) => whole(whole_units(a)?.abs()),
        ("sign", [Known::Number { units, .. }]) => whole(units.signum()),
        ("clamp", [number, low, high]) => {
            if is_true(BinaryOp::Less, number, low)? {
                return Some(low.clone());
            }

            if is_true(BinaryOp::Greater, number, high)? {
                return Some(high.clone());
            }

            Some(number.clone())
        }
        ("pow", [base, exponent]) => whole(power(whole_units(base)?, whole_units(exponent)?)?),
        ("checked_add", [a, b]) => checked(whole_units(a)?.checked_add(whole_units(b)?)?),
        ("checked_subtract", [a, b]) => checked(whole_units(a)?.checked_sub(whole_units(b)?)?),
        ("checked_multiply", [a, b]) => checked(whole_units(a)?.checked_mul(whole_units(b)?)?),
        _ => None,
    }
}

/// What calling the `Std.Numbers` function `name` with these known
/// arguments does that has no answer, if anything: `abs` of the lowest
/// whole number, `pow` past 32 bits or with an exponent below zero, and
/// `clamp` with its low above its high.
pub(crate) fn number_fault(name: &str, arguments: &[Known]) -> Option<Fault> {
    let outcome = match (name, arguments) {
        ("abs", [a]) if whole_units(a)? == INT_LOW => Outcome::DoesNotFit(-INT_LOW),
        ("clamp", [_, low, high]) if is_true(BinaryOp::Greater, low, high)? => {
            Outcome::LowAboveHigh
        }
        ("pow", [_, exponent]) if whole_units(exponent)? < 0 => Outcome::ExponentBelowZero,
        ("pow", [base, exponent]) => match power(whole_units(base)?, whole_units(exponent)?) {
            Some(answer) if fits(answer) => return None,
            Some(answer) => Outcome::DoesNotFit(answer),
            None => Outcome::FarTooBig,
        },
        _ => return None,
    };

    let written: Vec<String> = arguments.iter().map(Known::describe).collect();

    Some(Fault {
        operation: format!("{name}({})", written.join(", ")),
        outcome,
    })
}

/// A value for argument `index` of `name`, not known, that leaves the
/// answer to the known arguments `given`: the number `clamp` keeps between
/// its low and high, which breaks nothing itself, and `pow`'s base, whose
/// exponent alone can be below zero.
pub(crate) fn stand_in(name: &str, index: usize, given: &[Option<Known>]) -> Option<Known> {
    match (name, index) {
        ("clamp", 0) => given.get(1).cloned().flatten(),
        ("pow", 0) => Some(Known::int(1)),
        _ => None,
    }
}

/// `base` to the power of `exponent`, which is 0 or more; `None` when it is
/// too far from zero to work out.
fn power(base: i128, exponent: i128) -> Option<i128> {
    if matches!(base, 0 | 1) && exponent > 0 {
        return Some(base);
    }

    if base == -1 {
        return Some(if exponent % 2 == 0 { 1 } else { -1 });
    }

    base.checked_pow(u32::try_from(exponent).ok()?)
}

/// The units of a known whole number, with no places.
fn whole_units(value: &Known) -> Option<i128> {
    match value {
        Known::Number { units, places: 0 } => Some(*units),
        _ => None,
    }
}

/// A known whole number, when it fits in 32 bits.
fn whole(units: i128) -> Option<Known> {
    fits(units).then_some(Known::Number { units, places: 0 })
}

/// What checked arithmetic gives: `Some` of the answer when it fits in 32
/// bits, and `None` when it doesn't.
fn checked(units: i128) -> Option<Known> {
    Some(Known::Option(whole(units).map(Box::new)))
}

/// Whether `a op b` holds, for a comparison of two known numbers.
fn is_true(op: BinaryOp, a: &Known, b: &Known) -> Option<bool> {
    match Known::binary(op, a.clone(), b.clone())? {
        Known::Bool(answer) => Some(answer),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powers_of_zero_and_one_work_out_for_any_exponent() {
        assert_eq!(power(0, 0), Some(1));
        assert_eq!(power(0, 5), Some(0));
        assert_eq!(power(1, 2_147_483_647), Some(1));
        assert_eq!(power(-1, 2_147_483_647), Some(-1));
        assert_eq!(power(2, 200), None);
    }

    #[test]
    fn checked_arithmetic_past_32_bits_gives_none() {
        let actual = number_function("checked_add", &[Known::int(2_147_483_647), Known::int(1)]);

        assert_eq!(actual, Some(Known::Option(None)));
    }
}
