//! Working out constraints on values known when the program is compiled. A
//! value whose constraint can be worked out and fails is an error found by
//! `croft check`; one that depends on what happens while the program runs is
//! left to the check in the generated C.

use std::collections::{HashMap, HashSet};

use croft_syntax::ast::{
    Argument, BinaryOp, ConstraintUse, Expr, ExprId, ExprKind, FieldValue, Parameter, Path,
    Statement, UnaryOp,
};
use croft_syntax::Span;

use super::data::expr_path;
use super::known::{fault, negate_fault, Fault, Known, Outcome};
use super::numbers::{number_fault, number_function};
use super::{Checker, Scope};
use crate::standard::standard_context_of;
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

/// How deep constraints and constants may refer to each other before the
/// answer is left to runtime, so one that refers to itself cannot loop.
const MAX_DEPTH: u32 = 32;

/// The known values of the names in scope, such as a constraint's parameters.
pub(crate) type Bindings = HashMap<String, Known>;

/// Arithmetic with no 32-bit answer, or a call breaking its parameter's
/// constraint, found while checking. One in a body is reported where it is
/// written. One found only by working out a call with known arguments is
/// reported at the call, once every body is checked, and only when it was
/// not reported in its body already, nor at the call that led to it.
#[derive(Default)]
pub(crate) struct Faults {
    /// The first fault found while a call is worked out, when one is.
    recording: Option<Option<FoundFault>>,
    /// The calls being worked out inside the one being recorded, outermost
    /// first, by where each is written.
    calls: Vec<Span>,
    /// The expressions whose fault has been reported where it is written.
    reported: HashSet<ExprId>,
    /// Faults found through calls: the call as written, where it is, and
    /// the fault.
    through_calls: Vec<(String, Span, FoundFault)>,
}

/// A fault found by working out a call.
#[derive(Clone)]
pub(crate) struct FoundFault {
    /// The expression that faults.
    id: ExprId,
    fault: Fault,
    /// Where the first call on the way to it is written, in the body of the
    /// function called, when it is not the expression itself.
    via: Option<Span>,
}

impl Faults {
    /// Notes that the fault in `id` has been reported where it is written;
    /// false when it already had been.
    pub(crate) fn report(&mut self, id: ExprId) -> bool {
        self.reported.insert(id)
    }

    /// Keeps a fault found by working out `call`, written at `span`.
    pub(crate) fn add_through_call(&mut self, call: String, span: Span, found: FoundFault) {
        self.through_calls.push((call, span, found));
    }

    /// Each fault found through a call and not reported in its body, as a
    /// message and where to report it, once each. One whose way leads
    /// through a call reported with the same fault is left to that call.
    pub(crate) fn unreported(&mut self) -> Vec<(String, Span)> {
        let through_calls = std::mem::take(&mut self.through_calls);

        let reported_at = |span: Span, id: ExprId| {
            (through_calls.iter()).any(|(_, at, found)| *at == span && found.id == id)
        };

        let mut seen = HashSet::new();

        (through_calls.iter())
            .filter(|(_, _, found)| !self.reported.contains(&found.id))
            .filter(|(_, _, found)| !found.via.is_some_and(|via| reported_at(via, found.id)))
            .map(|(call, span, found)| (found.fault.describe(call), *span))
            .filter(|(message, span)| seen.insert((message.clone(), span.line, span.column)))
            .collect()
    }
}

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
    /// when that depends on what happens while the program runs. `bindings`
    /// are the known values of the names its arguments can read, such as the
    /// other parameters of a call.
    pub(super) fn keeps_constraint(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        used: &ConstraintUse,
        value: &Known,
    ) -> Option<bool> {
        let arguments: Vec<Known> = (used.arguments.iter())
            .map(|argument| self.known_value(scope, bindings, argument))
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

    /// The first arithmetic with no 32-bit answer that calling `found` with
    /// these arguments works out, and the expression it is in.
    pub(super) fn call_fault(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        found: &Resolved,
        arguments: &[Argument],
    ) -> Option<FoundFault> {
        self.faults.borrow_mut().recording = Some(None);
        self.known_call_at(scope, bindings, (found, None), arguments, 0);

        self.faults.borrow_mut().recording.take().flatten()
    }

    /// Keeps `fault`, in the expression `id`, when a call is being worked
    /// out and it is the first found.
    fn note_fault(&self, id: ExprId, fault: Option<Fault>) {
        let (mut faults, Some(fault)) = (self.faults.borrow_mut(), fault) else {
            return;
        };

        let via = faults.calls.first().copied();

        if let Some(first @ None) = &mut faults.recording {
            *first = Some(FoundFault { id, fault, via });
        }
    }

    /// `known_value`, `depth` references deep: past `MAX_DEPTH` it gives up.
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
            // A set is in no order, so it is never compared as a list would be.
            ExprKind::List(_) if matches!(self.facts.types.get(&value.id), Some(Type::Set(_))) => {
                None
            }
            ExprKind::List(items) => items
                .iter()
                .map(known)
                .collect::<Option<_>>()
                .map(Known::List),
            ExprKind::Name(name) => match bindings.get(name) {
                Some(bound) => Some(bound.clone()),
                None if name == "None" => Some(Known::Option(None)),
                None => self.constant_value(scope, &expr_path(value)?, depth),
            },
            ExprKind::Field(receiver, member) if member.name == "length" => {
                known(receiver)?.length()
            }
            ExprKind::Field(receiver, member) if member.name == "is_empty" => {
                let length = known(receiver)?.length()?;

                Known::binary(BinaryOp::Equal, length, Known::int(0))
            }
            ExprKind::Field(receiver, member) if is_option_field(&member.name) => {
                let Known::Option(held) = known(receiver)? else {
                    return None;
                };

                Some(Known::Bool(held.is_some() == (member.name == "is_some")))
            }
            ExprKind::Field(receiver, member) => match known(receiver) {
                Some(record) => record.field(&member.name),
                None => self.constant_value(scope, &expr_path(value)?, depth),
            },
            ExprKind::Record(path, fields) => {
                let fields = self.known_fields(scope, bindings, fields, depth)?;

                Some(Known::Record {
                    name: path.text(),
                    fields,
                })
            }
            ExprKind::With(base, changed) => {
                let Known::Record { name, mut fields } = known(base)? else {
                    return None;
                };

                for (field, value) in self.known_fields(scope, bindings, changed, depth)? {
                    let kept = fields.iter_mut().find(|(kept, _)| *kept == field)?;
                    kept.1 = value;
                }

                Some(Known::Record { name, fields })
            }
            ExprKind::Unary(op, operand) => {
                let operand = known(operand)?;

                if *op == UnaryOp::Negate {
                    self.note_fault(value.id, negate_fault(&operand));
                }

                Known::unary(*op, operand)
            }
            ExprKind::Binary(BinaryOp::And, left, right) => {
                both(known(left), || known(right), false)
            }
            ExprKind::Binary(BinaryOp::Or, left, right) => both(known(left), || known(right), true),
            ExprKind::Binary(op, left, right) => {
                let (left, right) = (known(left)?, known(right)?);
                self.note_fault(value.id, fault(*op, &left, &right));

                Known::binary(*op, left, right)
            }
            ExprKind::Block(statements) => self.known_block(scope, bindings, statements, depth),
            ExprKind::Call(callee, arguments) => {
                if let Some(held) = self.known_unwrap_or(callee, arguments, &known) {
                    return Some(held);
                }

                let path = expr_path(callee)?;
                let found = self.find_path(scope, &path);

                if let Some(found) = found.clone().filter(is_pure_function) {
                    let call = Some((value.id, value.span));
                    return self.known_call_at(scope, bindings, (&found, call), arguments, depth);
                }

                let arguments: Vec<Known> = (arguments.iter())
                    .map(|argument| known(&argument.value))
                    .collect::<Option<_>>()?;

                if found.is_none() {
                    let name = path.text();

                    if name == "Some" {
                        let [held] = <[Known; 1]>::try_from(arguments).ok()?;

                        return Some(Known::Option(Some(Box::new(held))));
                    }

                    if standard_context_of(&name) == Some("Std.Numbers") {
                        self.note_fault(value.id, number_fault(&name, &arguments));

                        return number_function(&name, &arguments);
                    }
                }

                let (checked, rest) = arguments.split_first()?;

                self.constraint_answer(scope, &path, checked, rest, depth)
                    .map(Known::Bool)
            }
            _ => None,
        }
    }

    /// The known value of each field given in a record or `with`, in order.
    /// Shorthand, `{ day }`, is the name of the same name.
    fn known_fields(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        fields: &[FieldValue],
        depth: u32,
    ) -> Option<Vec<(String, Known)>> {
        (fields.iter())
            .map(|given| {
                let value = match &given.value {
                    Some(value) => self.known_at(scope, bindings, value, depth)?,
                    None => bindings.get(&given.name.name)?.clone(),
                };

                Some((given.name.name.clone(), value))
            })
            .collect()
    }

    /// What `option.unwrap_or(fallback)` gives, when `callee` is that method
    /// on an option whose value is known.
    fn known_unwrap_or(
        &self,
        callee: &Expr,
        arguments: &[Argument],
        known: &dyn Fn(&Expr) -> Option<Known>,
    ) -> Option<Known> {
        let (ExprKind::Field(receiver, method), [fallback]) = (&callee.kind, arguments) else {
            return None;
        };

        if method.name != "unwrap_or" {
            return None;
        }

        match known(receiver)? {
            Known::Option(Some(held)) => Some(*held),
            Known::Option(None) => known(&fallback.value),
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
        self.known_call_at(scope, bindings, (found, None), arguments, 0)
    }

    /// `known_call`, `depth` references deep: past `MAX_DEPTH` it gives up.
    /// `call` is the call's own expression when it is written in a body being
    /// worked out, so a parameter constraint its known arguments break is
    /// kept as a fault of the call that led there.
    fn known_call_at(
        &self,
        scope: &Scope,
        bindings: &Bindings,
        (found, call): (&Resolved, Option<(ExprId, Span)>),
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

        let Some((call, span)) = call else {
            return self.known_at(&owner, &given, function.body.as_ref()?, depth + 1);
        };

        self.note_broken_parameter(&owner, &given, (found, call), parameters);

        self.faults.borrow_mut().calls.push(span);
        let answer = function
            .body
            .as_ref()
            .and_then(|body| self.known_at(&owner, &given, body, depth + 1));
        self.faults.borrow_mut().calls.pop();

        answer
    }

    /// Keeps, as a fault of `call`, the first constraint without `or` on
    /// `parameters` that the known arguments in `given` break.
    fn note_broken_parameter(
        &self,
        owner: &Scope,
        given: &Bindings,
        (found, call): (&Resolved, ExprId),
        parameters: &[Parameter],
    ) {
        for parameter in parameters {
            let value = &given[&parameter.name.name];

            let broken = (parameter.constraints.iter())
                .filter(|constraint| constraint.error.is_none())
                .find(|constraint| {
                    self.keeps_constraint(owner, given, constraint, value) == Some(false)
                });

            let Some(constraint) = broken else {
                continue;
            };

            let written: Vec<String> = (parameters.iter())
                .map(|parameter| given[&parameter.name.name].describe())
                .collect();

            let fault = Fault {
                operation: format!("{}({})", found.name, written.join(", ")),
                outcome: Outcome::BreaksConstraint {
                    parameter: parameter.name.name.clone(),
                    constraint: constraint.text(),
                },
            };

            self.note_fault(call, Some(fault));
            return;
        }
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
            // A guard whose condition holds gives back its block.
            if let Statement::Expr(Expr {
                kind: ExprKind::If(condition, block),
                ..
            }) = line
            {
                match self.known_at(scope, &bindings, condition, depth)? {
                    Known::Bool(true) => return self.known_at(scope, &bindings, block, depth),
                    Known::Bool(false) => continue,
                    _ => return None,
                }
            }

            let Statement::Let { name, value, .. } = line else {
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

/// Whether `name` is a field every option has: `is_some` or `is_none`.
fn is_option_field(name: &str) -> bool {
    matches!(name, "is_some" | "is_none")
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
