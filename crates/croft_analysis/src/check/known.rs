//! Values the compiler knows before the program runs, such as `5000`, `"Ada"`
//! or `MAX_DOSE - 1`, and the arithmetic and comparisons constraints use on them.

use std::cmp::Ordering;

use croft_syntax::ast::{BinaryOp, UnaryOp};

/// A value known when the program is compiled.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Known {
    /// A whole number or a decimal, as a whole number of its smallest unit:
    /// `2.50` is 250 units with two places, and `7` is 7 with none.
    Number {
        units: i128,
        places: u32,
    },
    Bool(bool),
    Text(String),
    List(Vec<Known>),
}

impl Known {
    /// A known whole number.
    pub(crate) fn int(value: i64) -> Known {
        Known::Number {
            units: i128::from(value),
            places: 0,
        }
    }

    /// A decimal literal as written, such as `99.99` or `1_000.5`.
    pub(crate) fn decimal(text: &str) -> Option<Known> {
        let text: String = text.chars().filter(|c| *c != '_').collect();
        let (whole, part) = text.split_once('.').unwrap_or((&text, ""));
        let units = format!("{whole}{part}").parse().ok()?;
        let places = u32::try_from(part.len()).ok()?;

        Some(Known::Number { units, places })
    }

    /// How many characters or items it holds, for `.length` and `Length`.
    pub(crate) fn length(&self) -> Option<Known> {
        let length = match self {
            Known::Text(text) => text.chars().count(),
            Known::List(items) => items.len(),
            _ => return None,
        };

        Some(Known::int(i64::try_from(length).ok()?))
    }

    /// The value as written in Croft, for error messages.
    pub(crate) fn describe(&self) -> String {
        match self {
            Known::Number { units, places: 0 } => units.to_string(),
            Known::Number { units, places } => {
                let digits = format!("{:0>width$}", units.abs(), width = *places as usize + 1);
                let (whole, part) = digits.split_at(digits.len() - *places as usize);
                let sign = if *units < 0 { "-" } else { "" };

                format!("{sign}{whole}.{part}")
            }
            Known::Bool(value) => value.to_string(),
            Known::Text(text) => format!("{text:?}"),
            Known::List(items) => {
                let items: Vec<String> = items.iter().map(Known::describe).collect();

                format!("[{}]", items.join(", "))
            }
        }
    }

    /// `!` or `-` applied to a known value, or `None` when it doesn't apply or
    /// the result overflows.
    pub(crate) fn unary(op: UnaryOp, operand: Known) -> Option<Known> {
        match (op, operand) {
            (UnaryOp::Not, Known::Bool(value)) => Some(Known::Bool(!value)),
            (UnaryOp::Negate, Known::Number { units, places }) => Some(Known::Number {
                units: units.checked_neg()?,
                places,
            }),
            (UnaryOp::BitNot, Known::Number { units, places: 0 }) => Some(Known::Number {
                units: !units,
                places: 0,
            }),
            _ => None,
        }
    }

    /// Every binary operator but `&&` and `||`, which `evaluate` works out
    /// itself so that one side can decide the answer without the other.
    pub(crate) fn binary(op: BinaryOp, left: Known, right: Known) -> Option<Known> {
        if let Some(ordering) = compare(&left, &right) {
            let answer = match op {
                BinaryOp::Equal => Some(ordering == Ordering::Equal),
                BinaryOp::NotEqual => Some(ordering != Ordering::Equal),
                BinaryOp::Less => Some(ordering == Ordering::Less),
                BinaryOp::LessEqual => Some(ordering != Ordering::Greater),
                BinaryOp::Greater => Some(ordering == Ordering::Greater),
                BinaryOp::GreaterEqual => Some(ordering != Ordering::Less),
                _ => None,
            };

            if let Some(answer) = answer {
                return Some(Known::Bool(answer));
            }
        }

        let (Known::Number { .. }, Known::Number { .. }) = (&left, &right) else {
            return None;
        };

        let (left, right, places) = aligned(&left, &right)?;

        let units = match op {
            BinaryOp::Add => left.checked_add(right)?,
            BinaryOp::Subtract => left.checked_sub(right)?,
            BinaryOp::Multiply if places == 0 => left.checked_mul(right)?,
            BinaryOp::Divide if places == 0 => left.checked_div(right)?,
            BinaryOp::Remainder if places == 0 => left.checked_rem(right)?,
            BinaryOp::BitAnd if places == 0 => left & right,
            BinaryOp::BitOr if places == 0 => left | right,
            BinaryOp::BitXor if places == 0 => left ^ right,
            BinaryOp::ShiftLeft if places == 0 => left.checked_shl(u32::try_from(right).ok()?)?,
            BinaryOp::ShiftRight if places == 0 => left.checked_shr(u32::try_from(right).ok()?)?,
            _ => return None,
        };

        Some(Known::Number { units, places })
    }
}

/// Two numbers in the same number of places, so `2.5` and `3` compare and add.
fn aligned(left: &Known, right: &Known) -> Option<(i128, i128, u32)> {
    let (
        Known::Number {
            units: a,
            places: p,
        },
        Known::Number {
            units: b,
            places: q,
        },
    ) = (left, right)
    else {
        return None;
    };

    let places = (*p).max(*q);
    let a = a.checked_mul(10_i128.checked_pow(places - p)?)?;
    let b = b.checked_mul(10_i128.checked_pow(places - q)?)?;

    Some((a, b, places))
}

/// How two values of the same kind compare; `None` when they are not the same kind.
fn compare(left: &Known, right: &Known) -> Option<Ordering> {
    match (left, right) {
        (Known::Number { .. }, Known::Number { .. }) => {
            let (a, b, _) = aligned(left, right)?;

            Some(a.cmp(&b))
        }
        (Known::Text(a), Known::Text(b)) => Some(a.cmp(b)),
        (Known::Bool(a), Known::Bool(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_different_places_compare_by_value() {
        let price = Known::decimal("2.50").unwrap();

        let actual = Known::binary(BinaryOp::Less, price, Known::int(3));

        assert_eq!(actual, Some(Known::Bool(true)));
    }

    #[test]
    fn arithmetic_that_overflows_is_not_known() {
        let actual = Known::binary(
            BinaryOp::Multiply,
            Known::int(i64::MAX),
            Known::int(i64::MAX),
        );
        let actual = actual.and_then(|big| Known::binary(BinaryOp::Multiply, big, Known::int(4)));

        assert_eq!(actual, None);
    }

    #[test]
    fn decimals_are_described_as_written() {
        assert_eq!(Known::decimal("120.00").unwrap().describe(), "120.00");
        assert_eq!(Known::decimal("0.05").unwrap().describe(), "0.05");
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        assert_eq!(Known::Text("café".into()).length(), Some(Known::int(4)));
    }
}
