//! Arithmetic with no answer in 32 bits. A whole number has exactly 32 bits
//! on every machine, and a `Decimal` keeps its smallest units in one, so `+`,
//! `-` and `*` past them, and `/` or `%` by zero, would stop the program.
//! When the values are known that is an error found by `croft check`, as is
//! a `Std.Numbers` function such as `pow` with no answer.

use croft_syntax::ast::{Argument, BinaryOp, Expr, ExprId, UnaryOp};
use croft_syntax::Span;

use super::BodyChecker;
use crate::check::known::{fault, fits, negate_fault, Known};
use crate::check::numbers::{number_fault, stand_in};
use crate::standard::standard_context_of;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Reports `whole`, `left op right` giving `ty`, when what it works out
    /// is known and has no answer: by zero whenever the right side is known
    /// to be zero, and otherwise when both sides are known.
    pub(super) fn check_known_arithmetic(
        &mut self,
        whole: &Expr,
        (op, left, right): (BinaryOp, &Expr, &Expr),
        ty: &Type,
    ) {
        use BinaryOp::*;

        if !matches!(op, Add | Subtract | Multiply | Divide | Remainder) {
            return;
        }

        if !matches!(ty, Type::Int | Type::Decimal(_)) {
            return;
        }

        let Some(right_value) = self.known_local_value(right) else {
            return;
        };

        if matches!(op, Divide | Remainder) && is_zero(&right_value) {
            self.report_fault(whole, &format!("`{}` divides by zero", whole.text()));
            return;
        }

        let Some(left_value) = self.known_local_value(left) else {
            return;
        };

        if let Type::Decimal(places) = ty {
            self.check_known_decimal(whole, Known::binary(op, left_value, right_value), *places);
            return;
        }

        if let Some(fault) = fault(op, &left_value, &right_value) {
            self.report_fault(whole, &fault.describe(&whole.text()));
        }
    }

    /// Reports `whole`, `-operand`, when the operand is known to be the
    /// lowest whole number, whose negative needs 33 bits.
    pub(super) fn check_known_negation(&mut self, whole: &Expr, operand: &Expr, ty: &Type) {
        let Some(operand) = self.known_local_value(operand) else {
            return;
        };

        if let Type::Decimal(places) = ty {
            self.check_known_decimal(whole, Known::unary(UnaryOp::Negate, operand), *places);
            return;
        }

        if let Some(fault) = negate_fault(&operand) {
            self.report_fault(whole, &fault.describe(&whole.text()));
        }
    }

    /// Reports a whole-number literal past 32 bits. `2147483648` is allowed
    /// only after `-`, which `unary` checks itself.
    pub(super) fn check_int_literal(&mut self, number: i64, span: Span) {
        if !fits(i128::from(number)) {
            let message = format!("`{number}` doesn't fit in 32 bits, the size of an `Int`");
            self.error(message, span);
        }
    }

    /// Reports a known `Decimal` answer, `answer`, whose smallest units in
    /// `places` places don't fit in 32 bits.
    fn check_known_decimal(&mut self, whole: &Expr, answer: Option<Known>, places: u32) {
        let Some(Known::Number {
            units,
            places: given,
        }) = answer
        else {
            return;
        };

        let Some(scaled) = (places.checked_sub(given))
            .and_then(|more| 10_i128.checked_pow(more))
            .and_then(|scale| units.checked_mul(scale))
        else {
            return;
        };

        if fits(scaled) {
            return;
        }

        let answer = Known::Number {
            units,
            places: given,
        }
        .describe();

        let message = format!(
            "`{}` gives {answer}, which doesn't fit in a `Decimal<{places}>`",
            whole.text()
        );

        self.report_fault(whole, &message);
    }

    /// Reports a call to a `Std.Numbers` function, such as `pow(10, 10)`,
    /// whose arguments are known and that has no answer.
    pub(super) fn check_known_number_call(
        &mut self,
        id: ExprId,
        name: &str,
        arguments: &[Argument],
        span: Span,
    ) {
        if standard_context_of(name) != Some("Std.Numbers") {
            return;
        }

        let given: Vec<Option<Known>> = (arguments.iter())
            .map(|argument| self.known_local_value(&argument.value))
            .collect();

        let Some(known) = (0..given.len())
            .map(|index| {
                given[index]
                    .clone()
                    .or_else(|| stand_in(name, index, &given))
            })
            .collect::<Option<Vec<Known>>>()
        else {
            return;
        };

        let Some(mut fault) = number_fault(name, &known) else {
            return;
        };

        let written: Vec<String> = arguments.iter().map(|a| a.value.text()).collect();
        let written = format!("{name}({})", written.join(", "));

        // With a stand-in, what it works out is only the call as written.
        if given.contains(&None) {
            fault.operation = written.clone();
        }

        let message = fault.describe(&written);

        if self.checker.faults.get_mut().report(id) {
            self.error(message, span);
        }
    }

    /// Reports a fault in `whole` once, however many times it is checked.
    fn report_fault(&mut self, whole: &Expr, message: &str) {
        if self.checker.faults.get_mut().report(whole.id) {
            self.error(message, whole.span);
        }
    }

    /// Keeps any fault that working out a call to `found`, a function or
    /// translation, with these arguments would find, to be reported at the
    /// call once every body is checked, unless its body reports it already.
    pub(super) fn check_known_call_faults(
        &mut self,
        found: &Resolved,
        arguments: &[Argument],
        span: Span,
    ) {
        let bindings = self.known_locals();

        let Some(fault) = (self.checker).call_fault(&self.scope, &bindings, found, arguments)
        else {
            return;
        };

        let call = format!("{}({})", found.name, self.written_arguments(arguments));

        (self.checker.faults.get_mut()).add_through_call(call, span, fault);
    }
}

/// Whether a known value is the number zero, in any places.
fn is_zero(value: &Known) -> bool {
    matches!(value, Known::Number { units: 0, .. })
}
