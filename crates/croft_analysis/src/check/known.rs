//! Values the compiler knows before the program runs, such as `5000`, `"Ada"`
//! or `MAX_DOSE - 1`, and the arithmetic and comparisons constraints use on them.

use std::cmp::Ordering;

use croft_syntax::ast::{BinaryOp, UnaryOp};

/// The lowest whole number: an `Int` has exactly 32 bits on every machine.
pub(crate) const INT_LOW: i128 = i32::MIN as i128;
/// The highest whole number.
pub(crate) const INT_HIGH: i128 = i32::MAX as i128;

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
    /// `Some(value)`, or `None` when it holds nothing.
    Option(Option<Box<Known>>),
    /// A value with every field known, such as `Date { year: 2025, month:
    /// 2, day: 29 }`: its type as written, and its fields in order.
    Record {
        name: String,
        fields: Vec<(String, Known)>,
    },
}

/// Arithmetic on known whole numbers that has no answer in 32 bits, so the
/// program would stop there: `2147483647 + 1`, or `10 / 0`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fault {
    /// The operation on the known values, as written: `100000 * 100000`.
    pub(crate) operation: String,
    /// What goes wrong.
    pub(crate) outcome: Outcome,
}

/// Why a [`Fault`] has no answer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Outcome {
    /// `/` or `%` by zero.
    DividesByZero,
    /// The answer, which is outside the 32 bits of a whole number.
    DoesNotFit(i128),
    /// An answer so far outside the 32 bits that it isn't worked out.
    FarTooBig,
    /// `pow` with an exponent below zero.
    ExponentBelowZero,
    /// `clamp` with its low above its high.
    LowAboveHigh,
}

impl Fault {
    /// The fault as the end of a message about `written`, the expression or
    /// call it happens in: "divides by zero".
    pub(crate) fn describe(&self, written: &str) -> String {
        let Fault { operation, outcome } = self;

        match outcome {
            Outcome::DividesByZero if written == operation => format!("`{written}` divides by zero"),
            Outcome::DividesByZero => format!("`{written}` divides by zero, in `{operation}`"),
            Outcome::DoesNotFit(answer) if written == operation => {
                format!("`{written}` gives {answer}, which doesn't fit in 32 bits")
            }
            Outcome::DoesNotFit(answer) => format!(
                "`{written}` works out `{operation}`, which gives {answer} and doesn't fit in 32 bits"
            ),
            Outcome::FarTooBig if written == operation => {
                format!("`{written}` doesn't fit in 32 bits")
            }
            Outcome::FarTooBig => {
                format!("`{written}` works out `{operation}`, which doesn't fit in 32 bits")
            }
            Outcome::ExponentBelowZero if written == operation => {
                format!("`{written}` has an exponent below zero")
            }
            Outcome::ExponentBelowZero => {
                format!("`{written}` works out `{operation}`, which has an exponent below zero")
            }
            Outcome::LowAboveHigh if written == operation => {
                format!("`{written}` has a low above its high")
            }
            Outcome::LowAboveHigh => {
                format!("`{written}` works out `{operation}`, which has a low above its high")
            }
        }
    }
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
            Known::Option(Some(value)) => format!("Some({})", value.describe()),
            Known::Option(None) => "None".to_string(),
            Known::Record { name, fields } => {
                let fields: Vec<String> = (fields.iter())
                    .map(|(field, value)| format!("{field}: {}", value.describe()))
                    .collect();

                format!("{name} {{ {} }}", fields.join(", "))
            }
        }
    }

    /// The field of a known record named `name`.
    pub(crate) fn field(&self, name: &str) -> Option<Known> {
        let Known::Record { fields, .. } = self else {
            return None;
        };

        (fields.iter())
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.clone())
    }

    /// `!` or `-` applied to a known value, or `None` when it doesn't apply or
    /// the result overflows.
    pub(crate) fn unary(op: UnaryOp, operand: Known) -> Option<Known> {
        match (op, operand) {
            (UnaryOp::Not, Known::Bool(value)) => Some(Known::Bool(!value)),
            (UnaryOp::Negate, Known::Number { units, places: 0 }) if !fits(-units) => None,
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
        if let (Known::Option(a), Known::Option(b)) = (&left, &right) {
            let same = match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    Known::binary(BinaryOp::Equal, (**a).clone(), (**b).clone())?
                        == Known::Bool(true)
                }
                _ => false,
            };

            return match op {
                BinaryOp::Equal => Some(Known::Bool(same)),
                BinaryOp::NotEqual => Some(Known::Bool(!same)),
                _ => None,
            };
        }

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

        if fault(op, &left, &right).is_some() {
            return None;
        }

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
            BinaryOp::ShiftLeft if places == 0 => shifted(left, right, i32::wrapping_shl)?,
            BinaryOp::ShiftRight if places == 0 => shifted(left, right, i32::wrapping_shr)?,
            _ => return None,
        };

        Some(Known::Number { units, places })
    }
}

/// Whether a whole number fits in 32 bits.
pub(crate) fn fits(units: i128) -> bool {
    (INT_LOW..=INT_HIGH).contains(&units)
}

/// What `left op right`, on two known whole numbers, does that has no
/// answer in 32 bits, if anything: `+`, `-` and `*` past them, `/` of the
/// lowest number by -1, or `/` and `%` by zero.
pub(crate) fn fault(op: BinaryOp, left: &Known, right: &Known) -> Option<Fault> {
    let (
        Known::Number {
            units: a,
            places: 0,
        },
        Known::Number {
            units: b,
            places: 0,
        },
    ) = (left, right)
    else {
        return None;
    };

    let operation = format!("{} {} {}", left.describe(), op.symbol(), right.describe());

    if matches!(op, BinaryOp::Divide | BinaryOp::Remainder) && *b == 0 {
        return Some(Fault {
            operation,
            outcome: Outcome::DividesByZero,
        });
    }

    let answer = match op {
        BinaryOp::Add => a.checked_add(*b)?,
        BinaryOp::Subtract => a.checked_sub(*b)?,
        BinaryOp::Multiply => a.checked_mul(*b)?,
        BinaryOp::Divide => a.checked_div(*b)?,
        _ => return None,
    };

    (!fits(answer)).then_some(Fault {
        operation,
        outcome: Outcome::DoesNotFit(answer),
    })
}

/// What `-operand`, on a known whole number, does that has no answer in 32
/// bits: only `-(-2147483648)` does.
pub(crate) fn negate_fault(operand: &Known) -> Option<Fault> {
    let Known::Number { units, places: 0 } = operand else {
        return None;
    };

    let answer = units.checked_neg()?;

    (!fits(answer)).then(|| Fault {
        operation: format!("-({})", operand.describe()),
        outcome: Outcome::DoesNotFit(answer),
    })
}

/// `value` shifted by `amount` as the 32 bits of an `Int` are, through
/// `shift`; `None` for an amount outside 0 to 31.
fn shifted(value: i128, amount: i128, shift: fn(i32, u32) -> i32) -> Option<i128> {
    let amount = u32::try_from(amount).ok().filter(|amount| *amount < 32)?;
    let value = i32::try_from(value).ok()?;

    Some(i128::from(shift(value, amount)))
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
    fn arithmetic_past_32_bits_is_a_fault() {
        let actual = fault(BinaryOp::Add, &Known::int(2_147_483_647), &Known::int(1));

        assert_eq!(
            actual.map(|fault| fault.describe("2147483647 + 1")),
            Some("`2147483647 + 1` gives 2147483648, which doesn't fit in 32 bits".to_string())
        );
    }

    #[test]
    fn dividing_the_lowest_number_by_minus_one_is_a_fault() {
        let lowest = Known::int(-2_147_483_648);

        assert!(fault(BinaryOp::Divide, &lowest, &Known::int(-1)).is_some());
        assert!(fault(BinaryOp::Remainder, &lowest, &Known::int(-1)).is_none());
    }

    #[test]
    fn shifting_works_in_32_bits() {
        let actual = Known::binary(BinaryOp::ShiftLeft, Known::int(1), Known::int(31));

        assert_eq!(actual, Some(Known::int(-2_147_483_648)));
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
