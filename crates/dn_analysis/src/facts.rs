//! What the checker learned about each expression, kept for code generation.

use std::collections::HashMap;

use dn_syntax::ast::ExprId;

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
    /// A function, translation, constraint or service declared in dn.
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
