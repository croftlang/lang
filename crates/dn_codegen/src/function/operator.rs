//! Operators: arithmetic, comparison, `&&`, `||` and equality.

use dn_analysis::Type;
use dn_syntax::ast::{BinaryOp, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub(super) fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr, ty: &Type) -> String {
        use BinaryOp::*;
        if matches!(op, And | Or) {
            return self.logic(op == And, left, right);
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
}
