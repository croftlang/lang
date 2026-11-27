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
use crate::types::{Resolved, Type};
use croft_syntax::Span;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks each argument whose value is known against the constraints of
    /// the parameter it is given for. Arguments are matched to parameters as
    /// `check_arguments` matches them; mistakes there are reported there. A
    /// constraint that names another parameter, `high: Int where AtLeast(low)`,
    /// reads the value given for it, when that is known.
    pub(super) fn check_known_arguments(
        &mut self,
        arguments: &[Argument],
        parameters: &[(&str, &[ConstraintUse])],
        owner: &Scope<'a>,
        callee: &str,
    ) {
        let mut position = 0;

        let matched: Vec<(&Argument, Option<usize>)> = (arguments.iter())
            .map(|argument| {
                let index = match &argument.name {
                    Some(name) => {
                        (parameters.iter()).position(|(parameter, _)| *parameter == name.name)
                    }
                    None => {
                        position += 1;
                        Some(position - 1)
                    }
                };

                (argument, index)
            })
            .collect();

        let given: Bindings = (matched.iter())
            .filter_map(|(argument, index)| {
                let (name, _) = parameters.get((*index)?)?;
                Some((name.to_string(), self.known_local_value(&argument.value)?))
            })
            .collect();

        for (argument, index) in matched {
            let Some((name, constraints)) = index.and_then(|index| parameters.get(index)) else {
                continue;
            };

            let place = KnownPlace {
                owner,
                given: &format!("`{callee}` is given"),
                name,
                bindings: &given,
            };

            self.check_known_value_with(&argument.value, constraints, &place);
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

    /// Reports each invariant of the value `ty` that `value`, a record or
    /// `with` making one, is known to break.
    pub(super) fn check_known_invariants(&mut self, ty: &Type, value: &Expr) {
        let Type::Named(named) = ty else { return };
        let invariants = self.checker.value_invariants(named);

        if invariants.is_empty() {
            return;
        }

        let Some(known) = self.known_local_value(value) else {
            return;
        };

        let owner = self.checker.scope_of(named);

        for invariant in invariants {
            let constraint = &invariant.constraint;

            let keeps =
                (self.checker).keeps_constraint(&owner, &Bindings::new(), constraint, &known);

            if keeps != Some(false) {
                continue;
            }

            let message = format!(
                "`{}` breaks `{}`'s invariant `{}`",
                known.describe(),
                named.name,
                constraint.text()
            );

            self.error(message, value.span);
        }
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
        let place = KnownPlace {
            owner,
            given,
            name,
            bindings: &Bindings::new(),
        };

        self.check_known_value_with(value, constraints, &place);
    }

    /// `check_known_value`, with the known values of the other parameters
    /// a constraint can name. A constraint that names one says what it was
    /// given: "which breaks `AtLeast(low)`, with `low` given `5`".
    fn check_known_value_with(
        &mut self,
        value: &Expr,
        constraints: &[ConstraintUse],
        place: &KnownPlace<'_, 'a>,
    ) {
        if constraints.is_empty() {
            return;
        }

        let Some(known) = self.known_local_value(value) else {
            return;
        };

        // One with `or` is a failure the caller handles, not a mistake.
        for constraint in constraints.iter().filter(|c| c.error.is_none()) {
            let keeps =
                (self.checker).keeps_constraint(place.owner, place.bindings, constraint, &known);

            if keeps != Some(false) {
                continue;
            }

            let mut message = format!(
                "{} `{}` for `{}`, which breaks `{}`",
                place.given,
                known.describe(),
                place.name,
                constraint.text()
            );

            let named = named_parameters(constraint, place.bindings);

            if !named.is_empty() {
                message.push_str(&format!(", with {}", named.join(" and ")));
            }

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
            let keeps =
                (self.checker).keeps_constraint(&owner, &Bindings::new(), constraint, &result);

            if keeps != Some(false) {
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
    pub(super) fn written_arguments(&self, arguments: &[Argument]) -> String {
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
    pub(super) fn known_locals(&self) -> Bindings {
        (self.locals.iter())
            .flat_map(|scope| scope.iter())
            .filter_map(|(name, local)| Some((name.clone(), local.known.clone()?)))
            .collect()
    }
}

/// Where a known value goes: the scope its constraints are written in, how
/// messages say it was given, and the known values of the names they can read.
struct KnownPlace<'p, 'a> {
    owner: &'p Scope<'a>,
    given: &'p str,
    name: &'p str,
    bindings: &'p Bindings,
}

/// "`low` given `5`" for each name in `bindings` that the constraint's
/// arguments read, in the order they are written.
fn named_parameters(constraint: &ConstraintUse, bindings: &Bindings) -> Vec<String> {
    let mut names = Vec::new();

    for argument in &constraint.arguments {
        names_in(argument, &mut names);
    }

    let mut seen = HashSet::new();

    (names.into_iter())
        .filter(|name| seen.insert(name.clone()))
        .filter_map(|name| {
            let known = bindings.get(&name)?;
            Some(format!("`{name}` given `{}`", known.describe()))
        })
        .collect()
}

/// Adds each bare name `value` reads to `names`, in the order written.
pub(super) fn names_in(value: &Expr, names: &mut Vec<String>) {
    match &value.kind {
        ExprKind::Name(name) => names.push(name.clone()),
        ExprKind::Field(receiver, _) => names_in(receiver, names),
        ExprKind::Unary(_, operand) => names_in(operand, names),
        ExprKind::Binary(_, left, right) => {
            names_in(left, names);
            names_in(right, names);
        }
        ExprKind::Call(callee, arguments) => {
            // A function's own name is not a value it reads.
            if let ExprKind::Field(receiver, _) = &callee.kind {
                names_in(receiver, names);
            }

            for argument in arguments {
                names_in(&argument.value, names);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                names_in(item, names);
            }
        }
        _ => {}
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
            Statement::Let { value, .. } | Statement::Expr(value) => changed_in(value, changed),
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
