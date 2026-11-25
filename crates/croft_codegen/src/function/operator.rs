//! Operators: arithmetic, bits, comparison, `&&`, `||` and equality.

use croft_analysis::Type;
use croft_syntax::ast::{BinaryOp, Expr};

use super::Writer;
use crate::types::bounds::is_string;
use crate::types::constant::evaluate;

const MAX_SHIFT: i64 = croft_analysis::MAX_SHIFT;

impl<'g, 'a> Writer<'g, 'a> {
    pub(super) fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, ty: &Type) -> String {
        use BinaryOp::*;

        if matches!(op, And | Or) {
            return self.logic(op == And, left, right);
        }

        if matches!(op, ShiftLeft | ShiftRight) {
            return self.shift(op == ShiftLeft, left, right);
        }

        if let Type::Decimal(places) = ty {
            return self.decimal_arithmetic(op, left, right, *places);
        }

        let left_type = self.type_of(left, None);

        if is_string(&left_type) && matches!(op, Equal | NotEqual | Add) {
            return self.string_operator(op, left, right);
        }

        let left = self.value(left, &left_type);
        let right = self.value(right, &left_type);

        match op {
            Equal | NotEqual => {
                let equal = self.equal(&left, &right, &left_type);

                if op == Equal {
                    equal
                } else {
                    format!("(!{equal})")
                }
            }
            _ => {
                let symbol = match op {
                    Add => "+",
                    Subtract => "-",
                    Multiply => "*",
                    Divide => "/",
                    Remainder => "%",
                    Less => "<",
                    LessEqual => "<=",
                    Greater => ">",
                    GreaterEqual => ">=",
                    BitAnd => "&",
                    BitOr => "|",
                    BitXor => "^",
                    _ => unreachable!("handled above"),
                };

                format!("({left} {symbol} {right})")
            }
        }
    }

    /// `&&` and `||` only work out their right side when it is needed.
    pub(super) fn logic(&mut self, is_and: bool, left: &Expr, right: &Expr) -> String {
        let left = self.value(left, &Type::Bool);
        let mark = self.mark();
        let right_value = self.value(right, &Type::Bool);
        let steps = self.take_since(mark);
        let symbol = if is_and { "&&" } else { "||" };

        if steps.is_empty() {
            return format!("({left} {symbol} {right_value})");
        }

        let temp = self.temp(&Type::Bool);
        self.line(format!("{temp} = {left};"));
        self.open(format!("if ({}{temp})", if is_and { "" } else { "!" }));
        self.put_back(steps);
        self.line(format!("{temp} = {right_value};"));
        self.close();

        temp
    }

    /// A shift. Left shifts go through `unsigned long`, so shifting a negative
    /// number is defined in C. An amount that is not a constant is checked in
    /// debug builds, since C leaves shifting by too much undefined.
    fn shift(&mut self, is_left: bool, left: &Expr, right: &Expr) -> String {
        let value = self.value(left, &Type::Int);
        let constant = evaluate(self.gen.program, &self.gen.model, &self.context, right);

        let amount = match constant {
            Some(amount) => amount.to_string(),
            None => {
                let amount = self.value(right, &Type::Int);
                let temp = self.temp(&Type::Int);
                self.line(format!("{temp} = {amount};"));

                self.line(format!(
                    "CROFT_CHECK({temp} >= 0 && {temp} < {MAX_SHIFT}, \"shift by less than 0 or more than {}\");",
                    MAX_SHIFT - 1
                ));

                temp
            }
        };

        let name = if is_left {
            "CROFT_SHIFT_LEFT"
        } else {
            "CROFT_SHIFT_RIGHT"
        };

        format!("{name}({value}, {amount})")
    }

    /// `==`, `!=` and `+` on strings, read through views so a literal on
    /// either side is never copied.
    fn string_operator(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> String {
        let (left, left_type) = self.bounded_view(left);
        let (right, right_type) = self.bounded_view(right);

        match op {
            BinaryOp::Add => {
                let length = self.gen.bound(&left_type) + self.gen.bound(&right_type);
                let joined = self.gen.text_type(length);
                let temp = self.temp(&joined);
                self.line(format!(
                    "croft_string_join({left}, {right}, CROFT_OUT({temp}));"
                ));
                temp
            }
            BinaryOp::NotEqual => format!("(!croft_string_equal({left}, {right}))"),
            _ => format!("croft_string_equal({left}, {right})"),
        }
    }
}
