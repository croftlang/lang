//! Operators: arithmetic, bits, comparison, `&&`, `||` and equality.

use croft_analysis::Type;
use croft_syntax::ast::{BinaryOp, Expr};

use super::Writer;
use crate::types::bounds::{is_bytes, is_string};
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

        if is_bytes(&left_type) && op == Add {
            let (left, right, joined) = self.bytes_parts(left, right);
            let temp = self.temp(&joined);
            self.line(format!(
                "croft_bytes_join({left}, {right}, CROFT_BYTES_OUT({temp}));"
            ));
            return temp;
        }

        if matches!(left_type, Type::List(_)) && op == Add {
            return self.join_lists(left, right, ty);
        }

        let left_c = self.value(left, &left_type);
        let right_c = self.value(right, &left_type);
        let (left, right) = (self.widened_operation(op, left, right, left_c), right_c);

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

    /// The two sides of `+` on `Bytes`, as views, and the type with room
    /// for both.
    pub(super) fn bytes_parts(&mut self, left: &Expr, right: &Expr) -> (String, String, Type) {
        let (left, left_type) = self.eval(left, Some(&Type::Bytes));
        let (right, right_type) = self.eval(right, Some(&Type::Bytes));
        let length = self.gen.bound(&left_type) + self.gen.bound(&right_type);

        (
            format!("CROFT_BYTES_VIEW({left})"),
            format!("CROFT_BYTES_VIEW({right})"),
            self.gen.bytes_type(length),
        )
    }

    /// `first + second` on lists: the items of both, in a list with room
    /// for the strings of either, as many as a list of them holds.
    fn join_lists(&mut self, left: &Expr, right: &Expr, ty: &Type) -> String {
        let (first, first_type) = self.eval(left, Some(ty));
        let (second, second_type) = self.eval(right, Some(ty));
        let joined = self
            .gen
            .keep_bound(&self.gen.wider(&first_type, &second_type), ty);

        let (Type::List(item), Type::List(second_item)) = (&joined, &second_type) else {
            return "0".to_string();
        };

        let (item, second_item) = ((**item).clone(), (**second_item).clone());
        let capacity = self.gen.sizes.list_capacity(&item);
        let temp = self.temp(&joined);
        let index = self.index_variable();
        let slot = format!("{temp}.items[{temp}.length]");

        self.copy_into(&temp, &first, &first_type, &joined);

        self.open(format!(
            "for ({index} = 0; {index} < {second}.length; {index}++)"
        ));

        self.line(format!(
            "CROFT_CHECK({temp}.length < {capacity}, \"a list is full\");"
        ));

        self.open(format!("if ({temp}.length < {capacity})"));

        let given = format!("{second}.items[{index}]");
        self.copy_into(&slot, &given, &second_item, &item);
        self.line(format!("{temp}.length++;"));
        self.close();
        self.close();

        temp
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
