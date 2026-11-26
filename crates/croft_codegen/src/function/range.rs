//! Which whole numbers an expression can be. Code generation works out a
//! range for each `Int` it can: from literals and constants, the `Range` of a
//! field, parameter or loop counter, the length of a string or list, and the
//! arithmetic that joins them. A number with no range is a full `croft_int`.
//!
//! Arithmetic whose operands and result all fit a C `int`, which has at
//! least 16 bits, is written in `int`, which an 8-bit machine works out far
//! faster than a `long`. Arithmetic on narrow numbers whose result may not
//! fit is widened to `croft_int` first.

use croft_analysis::types::Resolved;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::Writer;
use crate::types::bounds::is_string;
use crate::types::constant::evaluate;
use crate::types::width::declared_range;

/// The fewest values a C `int` holds: `INT_MIN` and `INT_MAX` with 16 bits,
/// leaving out -32768, which one's complement machines cannot hold.
const INT_LOW: i64 = -32767;
const INT_HIGH: i64 = 32767;

/// A whole number as a C literal: an `int` when it fits one, else a `long`.
pub fn c_number(number: i64) -> String {
    if (INT_LOW..=INT_HIGH).contains(&number) {
        number.to_string()
    } else {
        format!("{number}L")
    }
}

/// `c` cast to `croft_int`, in brackets unless it is a name, a call, a
/// place or already in brackets, so the cast takes the whole of it.
fn as_long(c: &str) -> String {
    if is_atom(c) {
        format!("(croft_int) {c}")
    } else {
        format!("(croft_int) ({c})")
    }
}

fn is_atom(c: &str) -> bool {
    if c.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "_.[]".contains(ch))
    {
        return true;
    }

    // One bracketed group, such as `(a + b)`.
    let mut depth = 0;

    for (at, ch) in c.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }

        if depth == 0 && at + 1 < c.len() {
            return false;
        }
    }

    c.starts_with('(')
}

/// The lowest and highest values a whole number can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub low: i64,
    pub high: i64,
}

impl Span {
    pub fn new(low: i64, high: i64) -> Span {
        Span { low, high }
    }

    /// Whether every value fits a C `int` on every machine.
    pub fn fits_int(self) -> bool {
        self.low >= INT_LOW && self.high <= INT_HIGH
    }

    pub fn is_natural(self) -> bool {
        self.low >= 0
    }

    /// The largest distance from zero of any value.
    fn magnitude(self) -> i64 {
        self.low.unsigned_abs().max(self.high.unsigned_abs()) as i64
    }

    /// The range of `a op b` for every `a` in `self` and `b` in `other`, from
    /// the four corners, or `None` when one overflows.
    fn corners(self, other: Span, op: fn(i64, i64) -> Option<i64>) -> Option<Span> {
        let values = [
            op(self.low, other.low)?,
            op(self.low, other.high)?,
            op(self.high, other.low)?,
            op(self.high, other.high)?,
        ];

        Some(Span::new(
            *values.iter().min().expect("four values"),
            *values.iter().max().expect("four values"),
        ))
    }
}

/// The range of `left op right`, for an operator giving a number.
pub fn combine(op: BinaryOp, left: Span, right: Span) -> Option<Span> {
    use BinaryOp::*;

    match op {
        Add => left.corners(right, i64::checked_add),
        Subtract => left.corners(right, i64::checked_sub),
        Multiply => left.corners(right, i64::checked_mul),
        // C rounds a quotient toward zero, so it is never further from zero
        // than the number divided.
        Divide => {
            let most = left.magnitude();

            if left.is_natural() && right.is_natural() {
                Some(Span::new(0, left.high))
            } else {
                Some(Span::new(-most, most))
            }
        }
        // A remainder is nearer zero than the divisor, and has the sign of
        // the number divided.
        Remainder => {
            let most = (right.magnitude() - 1).max(0).min(left.magnitude());

            if left.is_natural() {
                Some(Span::new(0, most))
            } else {
                Some(Span::new(-most, most))
            }
        }
        BitAnd if left.is_natural() || right.is_natural() => {
            let high = match (left.is_natural(), right.is_natural()) {
                (true, true) => left.high.min(right.high),
                (true, false) => left.high,
                _ => right.high,
            };

            Some(Span::new(0, high))
        }
        BitOr | BitXor if left.is_natural() && right.is_natural() => {
            let bits = 64 - left.high.max(right.high).leading_zeros();
            Some(Span::new(0, (1i64 << bits) - 1))
        }
        _ => None,
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// The values a whole-number expression can be, when they are known.
    pub fn range_of(&self, value: &Expr) -> Option<Span> {
        if let Some(constant) = evaluate(self.gen.program, &self.gen.model, &self.context, value) {
            return Some(Span::new(constant, constant));
        }

        if self.type_of(value, None) != Type::Int {
            return None;
        }

        match &value.kind {
            ExprKind::Name(name) => match self.gen.facts.meaning(value.id) {
                Some(Meaning::OwnField) => {
                    let owner = self.own.as_ref()?.owner.as_ref()?;
                    self.gen.field_range(owner, name)
                }
                _ => {
                    let (c, _) = self.lookup(name)?;
                    self.ranges.get(&c).copied()
                }
            },
            ExprKind::Field(inner, field) => {
                if self.gen.facts.meaning(value.id) != Some(&Meaning::Property) {
                    return None;
                }

                match (self.type_of(inner, None), field.name.as_str()) {
                    (Type::Named(owner), name) => self.gen.field_range(&owner, name),
                    (ty, "length") => Some(Span::new(0, self.most_held(&ty)?)),
                    _ => None,
                }
            }
            ExprKind::Unary(UnaryOp::Negate, operand) => {
                let inner = self.range_of(operand)?;
                Some(Span::new(-inner.high, -inner.low))
            }
            ExprKind::Unary(UnaryOp::BitNot, operand) => {
                let inner = self.range_of(operand)?;
                Some(Span::new(-inner.high - 1, -inner.low - 1))
            }
            ExprKind::Binary(op, left, right) => {
                combine(*op, self.range_of(left)?, self.range_of(right)?)
            }
            _ => None,
        }
    }

    /// The most items or characters a value of `ty` holds.
    fn most_held(&self, ty: &Type) -> Option<i64> {
        let sizes = &self.gen.sizes;

        match ty {
            ty if is_string(ty) => Some(self.gen.bound(ty)),
            Type::Bytes | Type::SizedBytes(_) => Some(self.gen.bound(ty)),
            Type::List(_) | Type::SizedList(..) => Some(self.gen.capacity(ty)),
            Type::Map(..) => Some(sizes.map_capacity(ty)),
            _ => None,
        }
    }

    /// A counter for the items of a list, a map or a string, which has no
    /// more than a `croft_u16` length can count.
    pub fn index_variable(&mut self) -> String {
        self.raw_variable("index", "croft_u16")
    }

    /// The length of a string, list, map or `Bytes` as a number. It is read
    /// as an `int` when every length it can be fits one.
    pub fn length_of(&self, c: &str, ty: &Type) -> String {
        match self.most_held(ty) {
            Some(most) if most <= INT_HIGH => format!("((int) {c}.length)"),
            _ => format!("((croft_int) {c}.length)"),
        }
    }

    /// `c`, the left of `left op right`, as a `croft_int` when the
    /// operation must be widened to one. A number with no known range is a
    /// `croft_int` already, so the work is done in `long` anyway. Otherwise
    /// both sides are narrow and C works in `int`, which is too small unless
    /// both sides fit one, and the result does too.
    pub fn widened_operation(&self, op: BinaryOp, left: &Expr, right: &Expr, c: String) -> String {
        let (Some(left), Some(right)) = (self.range_of(left), self.range_of(right)) else {
            return c;
        };

        let compares = matches!(
            op,
            BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
        );

        let fits = left.fits_int()
            && right.fits_int()
            && (compares || combine(op, left, right).is_some_and(Span::fits_int));

        if fits {
            c
        } else {
            as_long(&c)
        }
    }

    /// `c`, the operand of `-` or `~` in `whole`, as a `croft_int` when the
    /// operand is narrow and the result may not fit an `int`.
    pub fn widened(&self, c: String, whole: &Expr, operand: &Expr) -> String {
        match (self.range_of(operand), self.range_of(whole)) {
            (None, _) => c,
            (Some(_), Some(result)) if result.fits_int() => c,
            _ => as_long(&c),
        }
    }

    /// Records the values a C variable or parameter can hold.
    pub fn note_range(&mut self, c: &str, range: Span) {
        self.ranges.insert(c.to_string(), range);
    }
}

impl<'a> crate::Generator<'a> {
    /// The values a whole-number field can hold, from its `Range`.
    pub fn field_range(&self, owner: &Resolved, field: &str) -> Option<Span> {
        let fields = self.model.fields(owner);
        let (_, ty, declared) = fields.into_iter().find(|(name, _, _)| name == field)?;

        if ty != Type::Int {
            return None;
        }

        let (low, high) = declared_range(
            self.program,
            &self.model,
            &owner.context,
            &declared.constraints,
        )?;

        Some(Span::new(low, high))
    }
}
