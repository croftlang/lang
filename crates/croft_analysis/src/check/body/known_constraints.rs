//! Checking values known when the program is compiled against the
//! constraints they must keep, so a broken one is found by `croft check`.

use croft_syntax::ast::{
    Argument, ConstraintUse, Expr, ExprId, ExprKind, Field, FieldValue, FunctionDecl, Parameter,
    Statement,
};

use super::BodyChecker;
use std::collections::HashSet;

use crate::check::evaluate::Bindings;
use crate::check::known::Known;
use crate::check::Scope;
use crate::types::Resolved;
use croft_syntax::Span;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks each argument whose value is known against the constraints of
    /// the parameter it is given for. Arguments are matched to parameters as
    /// `check_arguments` matches them; mistakes there are reported there.
    pub(super) fn check_known_arguments(
        &mut self,
        arguments: &[Argument],
        parameters: &[(&str, &[ConstraintUse])],
        owner: &Scope<'a>,
        callee: &str,
    ) {
        let mut position = 0;

        for argument in arguments {
            let index = match &argument.name {
                Some(name) => {
                    (parameters.iter()).position(|(parameter, _)| *parameter == name.name)
                }
                None => {
                    position += 1;
                    Some(position - 1)
                }
            };

            let Some((name, constraints)) = index.and_then(|index| parameters.get(index)) else {
                continue;
            };

            let given = format!("`{callee}` is given");

            self.check_known_value(&argument.value, constraints, owner, &given, name);
        }
    }

    /// The name and constraints of each parameter, in order.
    pub(super) fn parameter_constraints(parameters: &[Parameter]) -> Vec<(&str, &[ConstraintUse])> {
        (parameters.iter())
            .map(|p| (p.name.name.as_str(), p.constraints.as_slice()))
            .collect()
    }

    /// The name and constraints of each field, in order, as a variant's or a
    /// single value's arguments are matched to them.
    pub(super) fn field_constraints(fields: &[Field]) -> Vec<(&str, &[ConstraintUse])> {
        (fields.iter())
            .map(|f| (f.name.name.as_str(), f.constraints.as_slice()))
            .collect()
    }

    /// Checks a field value given in a record or `with`, when it is known,
    /// against the constraints of the field it names in `named`. Shorthand,
    /// `{ lives }`, is the local name of the same name.
    pub(super) fn check_known_field(&mut self, named: &Resolved, given: &FieldValue) {
        let Some(field) = (self.checker.fields_of(named).unwrap_or(&[]).iter())
            .find(|field| field.name.name == given.name.name)
        else {
            return;
        };

        let shorthand;

        let value = match &given.value {
            Some(value) => value,
            None => {
                shorthand = Expr {
                    kind: ExprKind::Name(given.name.name.clone()),
                    span: given.name.span,
                    id: ExprId {
                        source: given.name.span.source,
                        index: u32::MAX,
                    },
                };

                &shorthand
            }
        };

        let owner = self.checker.scope_of(named);
        let given = format!("`{}` is given", named.name);

        self.check_known_value(value, &field.constraints, &owner, &given, &field.name.name);
    }

    /// Reports each constraint in `constraints`, written in `owner`, that
    /// `value` is known to break. `given` and `name` say where the value went:
    /// "`give` is given" and "mg".
    pub(super) fn check_known_value(
        &mut self,
        value: &Expr,
        constraints: &[ConstraintUse],
        owner: &Scope<'a>,
        given: &str,
        name: &str,
    ) {
        if constraints.is_empty() {
            return;
        }

        let Some(known) = self.known_local_value(value) else {
            return;
        };

        // One with `or` is a failure the caller handles, not a mistake.
        for constraint in constraints.iter().filter(|c| c.error.is_none()) {
            if self.checker.keeps_constraint(owner, constraint, &known) != Some(false) {
                continue;
            }

            let message = format!(
                "{given} `{}` for `{name}`, which breaks `{}`",
                known.describe(),
                constraint.text()
            );

            self.error(message, value.span);
        }
    }

    /// Checks what a call to `found`, a function or translation, gives back
    /// against its result's constraints, when the arguments are known.
    pub(super) fn check_known_result(
        &mut self,
        found: &Resolved,
        arguments: &[Argument],
        span: Span,
    ) {
        let owner = self.checker.scope_of(found);

        let Some(function) = (owner.context.functions.iter()).find(|f| f.name.name == found.name)
        else {
            return;
        };

        if function.return_constraints.is_empty() {
            return;
        }

        let bindings = self.known_locals();

        let Some(result) = self
            .checker
            .known_call(&self.scope, &bindings, found, arguments)
        else {
            return;
        };

        let written = self.written_arguments(arguments);

        for constraint in &function.return_constraints {
            if self.checker.keeps_constraint(&owner, constraint, &result) != Some(false) {
                continue;
            }

            let message = format!(
                "`{}({written})` gives back `{}`, which breaks `{}`",
                found.name,
                result.describe(),
                constraint.text()
            );

            self.error(message, span);
        }
    }

    /// Checks each constraint written as a line of `function`'s contract,
    /// such as `!numbers.is_empty`, against the arguments that are known. One
    /// with an error, `mg > 0 or Refused`, is a failure the caller handles, so
    /// it is left to the program.
    pub(super) fn check_known_contract(
        &mut self,
        function: &FunctionDecl,
        owner: &Scope<'a>,
        arguments: &[Argument],
        callee: &str,
        span: Span,
    ) {
        let must_hold: Vec<&Expr> = (function.preconditions.iter())
            .filter(|line| !matches!(line.kind, ExprKind::OrElse(..)))
            .collect();

        if must_hold.is_empty() {
            return;
        }

        let parameters = function.parameters.as_deref().unwrap_or(&[]);
        let mut bindings = Bindings::new();

        for (position, argument) in arguments.iter().enumerate() {
            let parameter = match &argument.name {
                Some(name) => parameters.iter().find(|p| p.name.name == name.name),
                None => parameters.get(position),
            };

            let known = self.known_local_value(&argument.value);

            if let (Some(parameter), Some(known)) = (parameter, known) {
                bindings.insert(parameter.name.name.clone(), known);
            }
        }

        for line in must_hold {
            if self.checker.known_value(owner, &bindings, line) != Some(Known::Bool(false)) {
                continue;
            }

            let message = format!(
                "`{callee}({})` breaks `{}`",
                self.written_arguments(arguments),
                line.text()
            );

            self.error(message, span);
        }
    }

    /// The arguments of a call as their known values, `...` where one is not
    /// known, for messages: `move_bed(3, ...)`.
    fn written_arguments(&self, arguments: &[Argument]) -> String {
        let written: Vec<String> = (arguments.iter())
            .map(|argument| {
                let value = (self.known_local_value(&argument.value))
                    .map_or("...".to_string(), |known| known.describe());

                match &argument.name {
                    Some(name) => format!("{}: {value}", name.name),
                    None => value,
                }
            })
            .collect();

        written.join(", ")
    }

    /// The value of `value` in this body, when it is known: literals,
    /// constants, and `let` names whose values are known.
    pub(super) fn known_local_value(&self, value: &Expr) -> Option<Known> {
        self.checker
            .known_value(&self.scope, &self.known_locals(), value)
    }

    /// The local names in scope whose values are known.
    fn known_locals(&self) -> Bindings {
        (self.locals.iter())
            .flat_map(|scope| scope.iter())
            .filter_map(|(name, local)| Some((name.clone(), local.known.clone()?)))
            .collect()
    }
}

/// Adds every name a `set` anywhere in `statements` changes, including in
/// nested blocks, loops and guards, to `changed`.
pub(super) fn changed_names(statements: &[Statement], changed: &mut HashSet<String>) {
    for statement in statements {
        match statement {
            Statement::Set(name, value) => {
                changed.insert(name.name.clone());
                changed_in(value, changed);
            }
            Statement::Let(_, _, value) | Statement::Expr(value) => changed_in(value, changed),
        }
    }
}

/// Adds every name a `set` inside `value` changes to `changed`.
fn changed_in(value: &Expr, changed: &mut HashSet<String>) {
    let mut each = |inner: &Expr| changed_in(inner, changed);

    match &value.kind {
        ExprKind::Block(statements) => changed_names(statements, changed),
        ExprKind::Loop(_, body) | ExprKind::Lambda(_, body) | ExprKind::Try(body) => each(body),
        ExprKind::Unary(_, inner) | ExprKind::Field(inner, _) => each(inner),
        ExprKind::If(condition, body) => {
            each(condition);
            each(body);
        }
        ExprKind::Binary(_, left, right) | ExprKind::OrElse(left, right) => {
            each(left);
            each(right);
        }
        ExprKind::Match(subject, arms) => {
            each(subject);
            arms.iter().for_each(|arm| changed_in(&arm.value, changed));
        }
        ExprKind::Call(callee, arguments) => {
            each(callee);
            arguments.iter().for_each(|a| changed_in(&a.value, changed));
        }
        ExprKind::Native(_, arguments) => {
            arguments.iter().for_each(|a| changed_in(&a.value, changed))
        }
        ExprKind::Issue(issue) => {
            changed_in(&issue.target, changed);
            issue
                .arguments
                .iter()
                .for_each(|a| changed_in(&a.value, changed));
        }
        ExprKind::List(items) => items.iter().for_each(|item| changed_in(item, changed)),
        ExprKind::Map(entries) => entries.iter().for_each(|(key, item)| {
            changed_in(key, changed);
            changed_in(item, changed);
        }),
        ExprKind::Record(_, fields) | ExprKind::Emit(_, fields) => (fields.iter())
            .filter_map(|f| f.value.as_ref())
            .for_each(|v| changed_in(v, changed)),
        ExprKind::With(base, fields) => {
            changed_in(base, changed);
            (fields.iter())
                .filter_map(|f| f.value.as_ref())
                .for_each(|v| changed_in(v, changed))
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Name(_)
        | ExprKind::Break => {}
    }
}
