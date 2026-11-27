//! Loops and `break`, in services and adapters only.

use croft_syntax::ast::{Expr, ExprKind, LoopCounter};
use croft_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `loop`, in services and adapters only, with an optional counter.
    pub(super) fn check_loop(
        &mut self,
        counter: Option<&LoopCounter>,
        block: &Expr,
        span: Span,
    ) -> Type {
        if !matches!(self.kind, BodyKind::Service | BodyKind::Adapter(_)) {
            let message =
                "only services and adapters can loop; use a list method such as `map` here";
            self.error(message, span);
        }

        let ExprKind::Block(statements) = &block.kind else {
            return Type::Unit;
        };

        self.nested(|body| {
            if let Some(counter) = counter {
                body.check_counter(counter);
            }

            body.loops += 1;
            body.block(statements, None);
            body.loops -= 1;

            if let Some(counter) = counter {
                body.check_loop_growth(counter, statements);
            }
        });

        Type::Unit
    }

    /// A counter counts through the values a `Range` allows.
    pub(super) fn check_counter(&mut self, counter: &LoopCounter) {
        let has_range = counter
            .constraints
            .iter()
            .any(|constraint| !constraint.each && constraint.name.text() == "Range");

        if !has_range {
            let message = "a counted loop needs a `Range` constraint, such as `where Range(0, 9)`";
            self.error(message, counter.name.span);
        }

        for constraint in &counter.constraints {
            for argument in &constraint.arguments {
                self.expect(argument, &Type::Int);
            }
        }

        let place = crate::check::constraints::ConstraintPlace::LoopCounter;
        self.checker
            .reject_constraint_errors(&counter.constraints, place);

        self.bind(
            &counter.name.name,
            Type::Int,
            false,
            Some(counter.name.span),
        );
    }

    /// `break` is only allowed inside a loop.
    pub(super) fn check_break(&mut self, span: Span) -> Type {
        if self.loops == 0 {
            self.error("`break` can only be used inside a loop", span);
        }

        Type::Unit
    }
}
