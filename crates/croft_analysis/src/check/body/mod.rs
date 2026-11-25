//! Type checking bodies: every expression gets a type, and each kind of body
//! may only do what its kind allows.

mod adapter;
mod arguments;
mod block;
mod builtin;
mod bytes;
mod call;
mod decimal;
mod effect;
mod entry;
mod expect;
mod expression;
mod failure;
mod field;
mod generic_call;
mod issue;
mod list;
mod map;
mod method;
mod name;
mod option;
mod pattern;
mod port;
mod property;
mod record;
mod repeat;
mod setup;
mod string;
mod type_path;
mod variant;

use std::collections::HashMap;

use croft_syntax::ast::{AdapterDecl, AggregateDecl, Identifier};
use croft_syntax::Span;

use super::{Checker, Scope};
use crate::types::Type;

/// What kind of body is being checked, which decides what it may do.
#[derive(Clone, Copy)]
pub(super) enum BodyKind<'a> {
    /// A function or translation: pure, no ports.
    Function,
    Constraint,
    Constant,
    /// A command changes its aggregate's fields with `set` and raises its events.
    Command(&'a AggregateDecl),
    /// A service uses ports, loads and saves aggregates and can loop.
    Service,
    Policy,
    /// An adapter operation changes its adapter's fields and calls native code.
    Adapter(&'a AdapterDecl),
}

/// A name a body can see: a parameter, `let` name, match binding or lambda parameter.
pub(super) struct Local {
    pub(super) ty: Type,
    pub(super) settable: bool,
    /// Where the name is bound, when it is written in the source.
    pub(super) defined: Option<Span>,
}

pub(super) struct BodyChecker<'c, 'a> {
    checker: &'c mut Checker<'a>,
    scope: Scope<'a>,
    kind: BodyKind<'a>,
    locals: Vec<HashMap<String, Local>>,
    loops: usize,
    /// What the body gives back; a guard that returns a value must match it.
    result: Type,
    /// What `?` and `or` may fail with. `None` when the body cannot fail.
    error: Option<Type>,
}

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn new(
        checker: &'c mut Checker<'a>,
        scope: Scope<'a>,
        kind: BodyKind<'a>,
        result: Type,
    ) -> Self {
        let error = match &result {
            Type::Result(_, error) => Some((**error).clone()),
            _ => None,
        };

        let locals = vec![HashMap::new()];

        BodyChecker {
            checker,
            scope,
            kind,
            locals,
            loops: 0,
            result,
            error,
        }
    }

    pub(super) fn error(&mut self, message: impl Into<String>, span: Span) {
        self.checker.error(message, span);
    }

    /// Brings a local name into scope. `defined` is where it is written, and
    /// is noted for editors.
    pub(super) fn bind(&mut self, name: &str, ty: Type, settable: bool, defined: Option<Span>) {
        if let Some(span) = defined {
            self.note_local(name, &ty, span, defined);
        }

        let scope = self.locals.last_mut().expect("a local scope");

        let local = Local {
            ty,
            settable,
            defined,
        };

        scope.insert(name.to_string(), local);
    }

    /// Notes where a local name is bound or used, for editors.
    pub(super) fn note_local(&mut self, name: &str, ty: &Type, span: Span, defined: Option<Span>) {
        self.checker.facts.locals.push(crate::facts::LocalName {
            span,
            name: name.to_string(),
            ty: ty.clone(),
            defined,
        });
    }

    /// Records a field, operation or method used at `span`, for editors.
    pub(super) fn note_member(
        &mut self,
        member: &Identifier,
        receiver: &Type,
        ty: &Type,
        kind: crate::facts::MemberKind,
        defined: Option<Span>,
    ) {
        if *receiver == Type::Unknown {
            return;
        }

        self.checker.facts.members.push(crate::facts::Member {
            span: member.span,
            name: member.name.clone(),
            receiver: receiver.clone(),
            ty: ty.clone(),
            kind,
            defined,
        });
    }

    /// Where a field of a named type is declared.
    pub(super) fn field_span(&self, ty: &Type, field: &str) -> Option<Span> {
        let Type::Named(named) = ty else {
            return None;
        };

        let fields = self.checker.fields_of(named)?;

        Some(fields.iter().find(|f| f.name.name == field)?.name.span)
    }

    pub(super) fn local(&self, name: &str) -> Option<&Local> {
        self.locals.iter().rev().find_map(|scope| scope.get(name))
    }

    /// Runs `check` with a new innermost scope for names.
    pub(super) fn nested<T>(&mut self, check: impl FnOnce(&mut Self) -> T) -> T {
        self.locals.push(HashMap::new());

        let result = check(self);

        self.locals.pop();
        result
    }
}
