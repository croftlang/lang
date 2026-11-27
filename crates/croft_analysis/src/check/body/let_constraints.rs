//! The constraints on a `let` name, as in `let items: List<Int> where
//! Length(0, 200) = []`: checked on its first value and on each value `set`
//! gives it, and required as a `Length` on a list, set or map that `set`
//! changes, since nothing else says how much room it needs.

use croft_syntax::ast::{
    ConstraintUse, Expr, ExprKind, FieldValue, Identifier, LoopCounter, Statement, TypeExpr,
};

use super::BodyChecker;
use crate::check::constraints::ConstraintPlace;
use crate::check::evaluate::Bindings;
use crate::check::known::Known;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks a `let`'s constraints: `each` only on a list, no `or Error`, a
    /// `Length` known when the program is compiled, and a first value that
    /// keeps them all when it is known.
    pub(super) fn check_let_constraints(
        &mut self,
        name: &Identifier,
        ty: &Type,
        constraints: &[ConstraintUse],
        value: &Expr,
        known: Option<&Known>,
    ) {
        let is_list = matches!(ty, Type::List(_));

        for constraint in constraints {
            if constraint.each && !is_list {
                self.error("`each` can only be used on a list", constraint.name.span());
            }

            self.checker.check_constraint_use(&self.scope, constraint);
        }

        self.checker
            .reject_constraint_errors(constraints, ConstraintPlace::Let);

        self.note_let_length(name, ty, constraints);

        let Some(known) = known else {
            return;
        };

        for constraint in constraints.iter().filter(|c| c.error.is_none()) {
            if self
                .checker
                .keeps_constraint(&self.scope, constraint, known)
                != Some(false)
            {
                continue;
            }

            let message = format!(
                "`{}` starts as `{}`, which breaks `{}`",
                name.name,
                known.describe(),
                constraint.text()
            );

            self.error(message, value.span);
        }
    }

    /// Notes the most a `let`'s `Length` allows, which sizes its list, map or
    /// string, and so must be known when the program is compiled.
    fn note_let_length(&mut self, name: &Identifier, ty: &Type, constraints: &[ConstraintUse]) {
        let Some(high) = length_of(constraints).and_then(|length| length.arguments.last()) else {
            return;
        };

        let sized = matches!(
            ty,
            Type::List(_) | Type::Map(..) | Type::Set(_) | Type::String | Type::Bytes
        );

        if !sized {
            return;
        }

        let longest = match self
            .checker
            .known_value(&self.scope, &Bindings::new(), high)
        {
            Some(Known::Number { units, places: 0 }) => i64::try_from(units.max(0)).ok(),
            _ => None,
        };

        let Some(longest) = longest else {
            let message = format!(
                "the `Length` of `{}` must be known when the program is compiled, \
                 since it says how much room to keep: use a number or a constant",
                name.name
            );

            self.error(message, high.span);
            return;
        };

        self.checker
            .facts
            .let_lengths
            .push((name.name.clone(), ty.clone(), longest));
    }

    /// A list, set or map that `set` changes needs a `Length`, so the program
    /// keeps room for as many items as it can grow to.
    pub(super) fn require_length(
        &mut self,
        name: &Identifier,
        written: Option<&TypeExpr>,
        ty: &Type,
        constraints: &[ConstraintUse],
    ) {
        let (what, inner) = match ty {
            Type::List(item) | Type::Set(item) => ("items", vec![&**item]),
            Type::Map(key, value) => ("entries", vec![&**key, &**value]),
            _ => return,
        };

        // A type with a mistake in it is reported where the mistake is.
        if inner.contains(&&Type::Unknown) || length_of(constraints).is_some() {
            return;
        }

        let written = written.map_or_else(|| ty.to_string(), TypeExpr::text);

        let message = format!(
            "`{name}` is changed with `set`, so say how many {what} it can hold with a \
             `Length`, such as `let {name}: {written} where Length(0, 100) = ...`",
            name = name.name
        );

        self.error(message, name.span);
    }

    /// Checks the value `set` gives a `let` name, when it is known, against
    /// the constraints the `let` wrote.
    pub(super) fn check_set_constraints(&mut self, name: &Identifier, value: &Expr) {
        let Some(local) = self.local(&name.name).filter(|local| local.settable) else {
            return;
        };

        if local.constraints.is_empty() {
            return;
        }

        let constraints = local.constraints.clone();

        let Some(known) = self.known_local_value(value) else {
            return;
        };

        for constraint in &constraints {
            if self
                .checker
                .keeps_constraint(&self.scope, constraint, &known)
                != Some(false)
            {
                continue;
            }

            let message = format!(
                "`{}` is set to `{}`, which breaks `{}`",
                name.name,
                known.describe(),
                constraint.text()
            );

            self.error(message, value.span);
        }
    }

    /// Reports a counted loop that adds to a list more times than its
    /// `Length` allows. Only a loop that always runs to the end counts: one
    /// with no `break`, `?`, `or`, guard or loop inside it, whose counter has
    /// just a `Range` known when the program is compiled, adding with
    /// `set items = items.add(...)` lines of its own block to a list that
    /// starts with a known number of items and is only ever added to.
    pub(super) fn check_loop_growth(&mut self, counter: &LoopCounter, statements: &[Statement]) {
        let Some(times) = self.times_counted(counter) else {
            return;
        };

        if statements.iter().any(statement_may_leave) {
            return;
        }

        let mut adds: Vec<(&Identifier, &Expr, i128)> = Vec::new();

        for statement in statements {
            let Statement::Set(name, value) = statement else {
                continue;
            };

            if !adds_one(value, &name.name) {
                continue;
            }

            match adds
                .iter_mut()
                .find(|(added, _, _)| added.name == name.name)
            {
                Some((_, _, count)) => *count += 1,
                None => adds.push((name, value, 1)),
            }
        }

        for (name, value, count) in adds {
            let Some(local) = self.local(&name.name).filter(|local| local.settable) else {
                continue;
            };

            let Some(start) = local.grows_from else {
                continue;
            };

            let Some(length) = length_of(&local.constraints).cloned() else {
                continue;
            };

            let Some(high) = (length.arguments.last()).and_then(|high| {
                self.checker
                    .known_value(&self.scope, &Bindings::new(), high)
            }) else {
                continue;
            };

            let Known::Number {
                units: high,
                places: 0,
            } = high
            else {
                continue;
            };

            let Some(least) = times
                .checked_mul(count)
                .and_then(|added| added.checked_add(start))
            else {
                continue;
            };

            if least <= high {
                continue;
            }

            let message = format!(
                "this loop runs {times} times and adds to `{}` each time, so it ends up \
                 with at least {least} items, more than `{}` allows",
                name.name,
                length.text()
            );

            self.error(message, value.span);
        }
    }

    /// How many times a counted loop runs, when its counter has only a
    /// `Range` whose ends are known.
    fn times_counted(&self, counter: &LoopCounter) -> Option<i128> {
        let [range] = counter.constraints.as_slice() else {
            return None;
        };

        if range.each || range.name.text() != "Range" {
            return None;
        }

        let [low, high] = range.arguments.as_slice() else {
            return None;
        };

        let whole = |value: &Expr| match self.known_local_value(value) {
            Some(Known::Number { units, places: 0 }) => Some(units),
            _ => None,
        };

        let times = whole(high)?.checked_sub(whole(low)?)?.checked_add(1)?;

        (times > 0).then_some(times)
    }
}

/// The `Length` among `constraints`, when there is one.
fn length_of(constraints: &[ConstraintUse]) -> Option<&ConstraintUse> {
    (constraints.iter()).find(|constraint| !constraint.each && constraint.name.text() == "Length")
}

/// Whether `value` is `name.add(...)`: the list `name` with one more item.
fn adds_one(value: &Expr, name: &str) -> bool {
    let ExprKind::Call(callee, arguments) = &value.kind else {
        return false;
    };

    let ExprKind::Field(receiver, method) = &callee.kind else {
        return false;
    };

    method.name == "add"
        && arguments.len() == 1
        && matches!(&receiver.kind, ExprKind::Name(receiver) if receiver == name)
}

/// Whether every `set` of `name` in `statements`, nested blocks included,
/// only adds one item to it, so it never gets shorter.
pub(super) fn only_adds_to(statements: &[Statement], name: &str) -> bool {
    statements.iter().all(|statement| match statement {
        Statement::Set(target, value) if target.name == name => {
            adds_one(value, name) && expression_only_adds_to(value, name)
        }
        Statement::Set(_, value) | Statement::Let { value, .. } | Statement::Expr(value) => {
            expression_only_adds_to(value, name)
        }
    })
}

/// Whether every `set` of `name` inside `value` only adds one item to it.
fn expression_only_adds_to(value: &Expr, name: &str) -> bool {
    children(value)
        .into_iter()
        .all(|child| expression_only_adds_to(child, name))
        && match &value.kind {
            ExprKind::Block(statements) => only_adds_to(statements, name),
            _ => true,
        }
}

/// Whether a statement can end a loop's turn, or the loop, before the end
/// of its block: a `break`, `?`, `or`, guard or loop anywhere in it.
fn statement_may_leave(statement: &Statement) -> bool {
    match statement {
        Statement::Let { value, .. } | Statement::Set(_, value) | Statement::Expr(value) => {
            may_leave(value)
        }
    }
}

/// Whether `value` holds a `break`, `?`, `or`, guard or loop.
fn may_leave(value: &Expr) -> bool {
    let leaves = matches!(
        value.kind,
        ExprKind::Break
            | ExprKind::Try(_)
            | ExprKind::OrElse(..)
            | ExprKind::If(..)
            | ExprKind::Loop(..)
    );

    if leaves {
        return true;
    }

    if let ExprKind::Block(statements) = &value.kind {
        return statements.iter().any(statement_may_leave);
    }

    children(value).into_iter().any(may_leave)
}

/// The values written for the fields of a record, `emit` or `with`.
fn field_values(fields: &[FieldValue]) -> Vec<&Expr> {
    fields.iter().filter_map(|f| f.value.as_ref()).collect()
}

/// The expressions directly inside `value`, apart from the statements of a block.
fn children(value: &Expr) -> Vec<&Expr> {
    match &value.kind {
        ExprKind::Field(inner, _)
        | ExprKind::Unary(_, inner)
        | ExprKind::Try(inner)
        | ExprKind::Lambda(_, inner)
        | ExprKind::Loop(_, inner) => vec![&**inner],
        ExprKind::Binary(_, left, right)
        | ExprKind::OrElse(left, right)
        | ExprKind::If(left, right) => vec![&**left, &**right],
        ExprKind::Call(callee, arguments) => std::iter::once(&**callee)
            .chain(arguments.iter().map(|a| &a.value))
            .collect(),
        ExprKind::Native(_, arguments) => arguments.iter().map(|a| &a.value).collect(),
        ExprKind::Match(subject, arms) => std::iter::once(&**subject)
            .chain(arms.iter().map(|arm| &arm.value))
            .collect(),
        ExprKind::Issue(issue) => std::iter::once(&*issue.target)
            .chain(issue.arguments.iter().map(|a| &a.value))
            .collect(),
        ExprKind::List(items) => items.iter().collect(),
        ExprKind::Map(entries) => entries.iter().flat_map(|(k, v)| [k, v]).collect(),
        ExprKind::Record(_, values) | ExprKind::Emit(_, values) => field_values(values),
        ExprKind::With(base, values) => std::iter::once(&**base)
            .chain(field_values(values))
            .collect(),
        ExprKind::Block(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Name(_)
        | ExprKind::Break => Vec::new(),
    }
}
