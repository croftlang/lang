//! What the checker learned about each expression, kept for code generation.

use std::collections::HashMap;

use b_syntax::ast::ExprId;
use b_syntax::Span;

use crate::types::{Resolved, Type};

/// What a name, field access or call refers to.
#[derive(Debug, Clone, PartialEq)]
pub enum Meaning {
    /// A parameter, `let` name, match binding, lambda parameter or loop counter.
    Local,
    /// A field of the command's aggregate or the adapter.
    OwnField,
    Const(Resolved),
    /// An error with no fields used as a value, such as `ExitBlocked`.
    Error(Resolved),
    /// A variant of an enum, or an error it includes: the enum, the variant's
    /// name, and the error when the variant is one.
    Variant(Resolved, String, Option<Resolved>),
    Unit,
    NoneValue,
    /// A function, translation, constraint or service declared in b.
    Declared(Resolved),
    /// `max`, `Some`, `Range` and the other built-in functions.
    Builtin(String),
    /// `receiver.method(...)` on a port, repository or built-in type.
    Method,
    /// `value.field`.
    Property,
    /// `issue ... on Room(id)`: load, change and save.
    IssueByIdentity(Resolved),
    /// `issue ... on room`: change a value already held.
    IssueOnValue(Resolved),
}

#[derive(Debug, Default)]
pub struct Facts {
    pub types: HashMap<ExprId, Type>,
    pub meanings: HashMap<ExprId, Meaning>,
    /// The type a value was written to have, where that differs from its own,
    /// as with the `Int` in `let price: Decimal<2> = 3`.
    pub wanted: HashMap<ExprId, Type>,
    /// Each name written in the source and the declaration it refers to, for
    /// editors: hover and go to definition.
    pub references: Vec<Reference>,
    /// Each place a local name is bound or used, for editors.
    pub locals: Vec<LocalName>,
    /// Each field, operation or method named after a value, such as the
    /// `player` in `room.player`, and the field names in records, for editors.
    pub members: Vec<Member>,
}

/// A name as written, such as the `Room` in `Room.Error`, and what it names.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub span: Span,
    pub name: String,
    pub target: Resolved,
}

/// A parameter, `let` name, match binding, lambda parameter or loop counter,
/// where it is bound or used.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalName {
    pub span: Span,
    pub name: String,
    pub ty: Type,
    /// Where the name is bound. `None` for names a body gets without writing
    /// them, such as a policy's `event`.
    pub defined: Option<Span>,
}

impl Facts {
    pub fn type_of(&self, id: ExprId) -> &Type {
        self.types.get(&id).unwrap_or(&Type::Unknown)
    }

    /// The type a value is kept as: what it was written to have, or its own.
    pub fn kept_as(&self, id: ExprId) -> &Type {
        self.wanted.get(&id).unwrap_or_else(|| self.type_of(id))
    }

    pub fn meaning(&self, id: ExprId) -> Option<&Meaning> {
        self.meanings.get(&id)
    }
}

/// A field, port operation or built-in method, where it is used.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub span: Span,
    pub name: String,
    /// What it belongs to: `Room` for `room.player`, `Screen` for `screen.clear()`.
    pub receiver: Type,
    /// A field's type, or what a call gives back.
    pub ty: Type,
    pub kind: MemberKind,
    /// Where it is declared. `None` for what the language provides, such as
    /// `text.length` or `rooms.find(id)`.
    pub defined: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Field,
    Operation,
}
