//! Arithmetic on `Decimal`s. Adding and subtracting work on the whole numbers
//! underneath; multiplying and dividing rescale and round half to even.

use croft_analysis::Type;
use croft_syntax::ast::{BinaryOp, Expr};

use super::Writer;
use crate::types::constant::scale;

impl<'g, 'a> Writer<'g, 'a> {
    pub(super) fn decimal_arithmetic(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        places: u32,
    ) -> String {
        let decimal = Type::Decimal(places);
        let unit = scale(places);
        let left_type = self.type_of(left, None);
        let right_type = self.type_of(right, None);
        let both = left_type == decimal && right_type == decimal;

        match op {
            BinaryOp::Multiply if !both => {
                let left = self.value(left, &left_type);
                let right = self.value(right, &right_type);
                format!("({left} * {right})")
            }
            BinaryOp::Multiply => {
                let left = self.value(left, &decimal);
                let right = self.value(right, &decimal);
                format!("croft_decimal_multiply({left}, {right}, {unit}L)")
            }
            BinaryOp::Divide if right_type == Type::Int => {
                let left = self.value(left, &decimal);
                let right = self.value(right, &Type::Int);
                format!("croft_divide_even({left}, {right})")
            }
            BinaryOp::Divide => {
                let left = self.value(left, &decimal);
                let right = self.value(right, &decimal);
                format!("croft_decimal_divide({left}, {right}, {unit}L)")
            }
            _ => {
                let symbol = if op == BinaryOp::Add { "+" } else { "-" };
                let left = self.value(left, &decimal);
                let right = self.value(right, &decimal);
                format!("({left} {symbol} {right})")
            }
        }
    }
}
