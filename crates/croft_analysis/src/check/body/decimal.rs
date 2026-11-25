//! `Decimal<N>`: literals typed by where they go, and arithmetic with rounding.

use croft_syntax::ast::{BinaryOp, Expr};
use croft_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `2.50` is a `Decimal<2>` where one is expected, and a `Float` anywhere else.
    pub(super) fn float(&mut self, text: &str, span: Span, expected: Option<&Type>) -> Type {
        let Some(Type::Decimal(places)) = expected else {
            return Type::Float;
        };

        let written = text.split_once('.').map_or(0, |(_, after)| after.len());

        if written > *places as usize {
            let message = format!("`{text}` has more than {places} decimal places");
            self.error(message, span);
        }

        Type::Decimal(*places)
    }

    /// Arithmetic where either side is a `Decimal`, or `None` if neither is.
    /// A `Decimal` works with an `Int` or a `Decimal` of the same places, and
    /// gives the `Decimal` back. Multiplying and dividing two `Decimal`s, or
    /// dividing by an `Int`, rounds half to even.
    pub(super) fn decimal_arithmetic(
        &mut self,
        op: BinaryOp,
        left_type: &Type,
        left: &Expr,
        right: &Expr,
    ) -> Option<Type> {
        let right_expected = matches!(left_type, Type::Decimal(_)).then_some(left_type);

        if right_expected.is_none() && *left_type != Type::Int {
            return None;
        }

        let right_type = self.expr(right, right_expected);

        let decimal = match (left_type, &right_type) {
            (Type::Decimal(a), Type::Decimal(b)) if a == b => *a,
            (Type::Decimal(places), Type::Int) => *places,
            (Type::Int, Type::Decimal(places)) => *places,
            (Type::Int, _) => return self.whole_number_arithmetic(&right_type, right),
            _ => {
                let message = format!("expected `{left_type}`, found `{right_type}`");
                self.error(message, right.span);
                return Some(left_type.clone());
            }
        };

        let joined = Type::Decimal(decimal);

        if op == BinaryOp::Remainder {
            self.error("a `Decimal` has no remainder", left.span);
        }

        if op == BinaryOp::Divide && *left_type == Type::Int {
            self.error("only a `Decimal` can be divided by a `Decimal`", left.span);
        }

        Some(joined)
    }

    /// `Int` arithmetic, once the right side has already been checked.
    fn whole_number_arithmetic(&mut self, right_type: &Type, right: &Expr) -> Option<Type> {
        if !self.accepts(&Type::Int, right_type, right) {
            let message = format!("expected `Int`, found `{right_type}`");
            self.error(message, right.span);
        }

        Some(Type::Int)
    }
}
