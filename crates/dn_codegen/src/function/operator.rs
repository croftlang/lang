//! Operators: arithmetic, bits, comparison, `&&`, `||` and equality.

use dn_analysis::Type;
use dn_syntax::ast::{BinaryOp, Expr};

use super::Writer;
use crate::types::constant::evaluate;

const MAX_SHIFT: i64 = dn_analysis::MAX_SHIFT;

impl<'g, 'a> Writer<'g, 'a> {
    pub(super) fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, ty: &Type) -> String {
        use BinaryOp::*;
        if matches!(op, And | Or) {
            return self.logic(op == And, left, right);
        }
        if matches!(op, ShiftLeft | ShiftRight) {
            return self.shift(op == ShiftLeft, left, right);
        }
        let left_type = self.type_of(left, None);
        let (left, _) = self.eval(left, None);
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
            Add if left_type == Type::String => {
                let temp = self.temp(ty);
                self.line(format!("dn_string_join(&{left}, &{right}, &{temp});"));
                temp
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
                    "DN_CHECK({temp} >= 0 && {temp} < {MAX_SHIFT}, \"shift by less than 0 or more than {}\");",
                    MAX_SHIFT - 1
                ));
                temp
            }
        };
        let name = if is_left {
            "DN_SHIFT_LEFT"
        } else {
            "DN_SHIFT_RIGHT"
        };
        format!("{name}({value}, {amount})")
    }
}
