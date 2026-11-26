//! Working out constraints on values known when the program is compiled. A
//! value whose constraint can be worked out and fails is an error found by
//! `croft check`; one that depends on what happens while the program runs is
//! left to the check in the generated C.

use std::collections::HashMap;

use croft_syntax::ast::{Argument, BinaryOp, ConstraintUse, Expr, ExprKind, Path, Statement};

use super::data::expr_path;
use super::known::Known;
use super::{Checker, Scope};
use crate::symbols::SymbolKind;
use crate::types::Resolved;

/// How deep constraints and constants may refer to each other before the
/// answer is left to runtime, so one that refers to itself cannot loop.
const MAX_DEPTH: u32 = 32;

/// The known values of the names in scope, such as a constraint's parameters.
pub(crate) type Bindings = HashMap<String, Known>;

impl<'a> Checker<'a> {
    /// The value of `value`, when it is known without running the program.
    pub(super) fn known_value(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        value: &Expr,
    ) -> Option<Known> {
        self.known_at(scope, bindings, value, 0)
    }

    /// Whether `value` keeps `used`, a constraint written in `scope`; `None`
    /// when that depends on what happens while the program runs.
    pub(super) fn keeps_constraint(
        &self,
        scope: &Scope,
        used: &ConstraintUse,
        value: &Known,
    ) -> Option<bool> {
        let arguments: Vec<Known> = (used.arguments.iter())
            .map(|argument| self.known_value(scope, &Bindings::new(), argument))
            .collect::<Option<_>>()?;

        if !used.each {
            return self.constraint_answer(scope, &used.name, value, &arguments, 0);
        }

        let Known::List(items) = value else {
            return None;
        };

        let mut answer = Some(true);

        for item in items {
            match self.constraint_answer(scope, &used.name, item, &arguments, 0) {
                Some(false) => return Some(false),
                None => answer = None,
                Some(true) => {}
            }
        }

        answer
    }

    fn known_at(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        value: &Expr,
        depth: u32,
    ) -> Option<Known> {
        if depth > MAX_DEPTH {
            return None;
        }

        let depth = depth + 1;
        let known = |inner: &Expr| self.known_at(scope, bindings, inner, depth);

        match &value.kind {
            ExprKind::Int(number) => Some(Known::int(*number)),
            ExprKind::Float(text) => Known::decimal(text),
            ExprKind::String(text) => Some(Known::Text(text.clone())),
            ExprKind::Bool(value) => Some(Known::Bool(*value)),
            ExprKind::List(items) => items
                .iter()
                .map(known)
                .collect::<Option<_>>()
                .map(Known::List),
            ExprKind::Name(name) => match bindings.get(name) {
                Some(bound) => Some(bound.clone()),
                None => self.constant_value(scope, &expr_path(value)?, depth),
            },
            ExprKind::Field(receiver, member) if member.name == "length" => {
                known(receiver)?.length()
            }
            ExprKind::Field(..) => self.constant_value(scope, &expr_path(value)?, depth),
            ExprKind::Unary(op, operand) => Known::unary(*op, known(operand)?),
            ExprKind::Binary(BinaryOp::And, left, right) => {
                both(known(left), || known(right), false)
            }
            ExprKind::Binary(BinaryOp::Or, left, right) => both(known(left), || known(right), true),
            ExprKind::Binary(op, left, right) => Known::binary(*op, known(left)?, known(right)?),
            ExprKind::Block(statements) => self.known_block(scope, bindings, statements, depth),
            ExprKind::Call(callee, arguments) => {
                let path = expr_path(callee)?;

                if let Some(found) = self.find_path(scope, &path).filter(is_pure_function) {
                    return self.known_call_at(scope, bindings, &found, arguments, depth);
                }

                let arguments: Vec<Known> = (arguments.iter())
                    .map(|argument| known(&argument.value))
                    .collect::<Option<_>>()?;

                let (checked, rest) = arguments.split_first()?;

                self.constraint_answer(scope, &path, checked, rest, depth)
                    .map(Known::Bool)
            }
            _ => None,
        }
    }

    /// What calling `found`, a function or translation, gives back, when its
    /// arguments are known in `scope` and its body can be worked out.
    pub(super) fn known_call(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        found: &Resolved,
        arguments: &[Argument],
    ) -> Option<Known> {
        self.known_call_at(scope, bindings, found, arguments, 0)
    }

    fn known_call_at(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        found: &Resolved,
        arguments: &[Argument],
        depth: u32,
    ) -> Option<Known> {
        let owner = self.scope_of(found);
        let function = owner
            .context
            .functions
            .iter()
            .find(|f| f.name.name == found.name)?;

        if !function.type_parameters.is_empty() {
            return None;
        }

        let parameters = function.parameters.as_deref().unwrap_or(&[]);
        let mut given = Bindings::new();

        for (position, argument) in arguments.iter().enumerate() {
            let parameter = match &argument.name {
                Some(name) => parameters.iter().find(|p| p.name.name == name.name)?,
                None => parameters.get(position)?,
            };

            let value = self.known_at(scope, bindings, &argument.value, depth + 1)?;
            given.insert(parameter.name.name.clone(), value);
        }

        if given.len() != parameters.len() {
            return None;
        }

        self.known_at(&owner, &given, function.body.as_ref()?, depth + 1)
    }

    /// A block of `let` lines ending in its value. One that changes a name or
    /// takes a step before its last line is left to runtime.
    fn known_block(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        statements: &[Statement],
        depth: u32,
    ) -> Option<Known> {
        let mut bindings = bindings.clone();
        let (last, lines) = statements.split_last()?;

        for line in lines {
            let Statement::Let(name, _, value) = line else {
                return None;
            };

            let value = self.known_at(scope, &bindings, value, depth)?;
            bindings.insert(name.name.clone(), value);
        }

        let Statement::Expr(value) = last else {
            return None;
        };

        self.known_at(scope, &bindings, value, depth)
    }

    /// The value of a constant such as `MAX_DOSE` or `Clinic.MAX_DOSE`.
    fn constant_value(&self, scope: &Scope, path: &Path, depth: u32) -> Option<Known> {
        let found = self
            .find_path(scope, path)
            .filter(|found| found.kind == SymbolKind::Const)?;
        let owner = self.scope_of(&found);
        let constant = owner
            .context
            .consts
            .iter()
            .find(|c| c.name.name == found.name)?;

        self.known_at(&owner, &Bindings::new(), &constant.value, depth)
    }

    /// Whether `value` keeps the constraint named `name` with these arguments,
    /// working out a declared constraint's body with its parameters bound.
    fn constraint_answer(
        &self,
        scope: &Scope,
        name: &Path,
        value: &Known,
        arguments: &[Known],
        depth: u32,
    ) -> Option<bool> {
        let Some(found) = self.find_path(scope, name) else {
            return builtin_answer(&name.text(), value, arguments);
        };

        if found.kind != SymbolKind::Constraint {
            return None;
        }

        let owner = self.scope_of(&found);
        let declared = owner
            .context
            .constraints
            .iter()
            .find(|c| c.name.name == found.name)?;

        if declared.parameters.len() != arguments.len() + 1 {
            return None;
        }

        let values = std::iter::once(value).chain(arguments);

        let bindings: Bindings = (declared.parameters.iter())
            .zip(values)
            .map(|(parameter, value)| (parameter.name.name.clone(), value.clone()))
            .collect();

        match self.known_at(&owner, &bindings, &declared.body, depth)? {
            Known::Bool(answer) => Some(answer),
            _ => None,
        }
    }
}

/// Functions and translations are pure, so calling one with known arguments
/// can be worked out; services cannot, since they use ports.
fn is_pure_function(found: &Resolved) -> bool {
    matches!(found.kind, SymbolKind::Function | SymbolKind::Translation)
}

/// `Range(low, high)`, `Length(exactly)` and `Length(low, high)`.
fn builtin_answer(name: &str, value: &Known, arguments: &[Known]) -> Option<bool> {
    let within = |value: Known, low: &Known, high: &Known| -> Option<bool> {
        let above = Known::binary(BinaryOp::GreaterEqual, value.clone(), low.clone())?;
        let below = Known::binary(BinaryOp::LessEqual, value, high.clone())?;

        Some(above == Known::Bool(true) && below == Known::Bool(true))
    };

    match (name, arguments) {
        ("Range", [low, high]) => within(value.clone(), low, high),
        ("Length", [exactly]) => within(value.length()?, exactly, exactly),
        ("Length", [low, high]) => within(value.length()?, low, high),
        _ => None,
    }
}

/// `&&` when `decides` is false, `||` when it is true: one side with that
/// value is the answer, even when the other side is not known.
fn both(
    left: Option<Known>,
    right: impl FnOnce() -> Option<Known>,
    decides: bool,
) -> Option<Known> {
    if left == Some(Known::Bool(decides)) {
        return left;
    }

    let right = right();

    if right == Some(Known::Bool(decides)) {
        return right;
    }

    match (left, right) {
        (Some(Known::Bool(_)), Some(Known::Bool(_))) => Some(Known::Bool(!decides)),
        _ => None,
    }
}
