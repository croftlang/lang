//! The types a body's expressions can have.

use std::fmt;

use crate::symbols::SymbolKind;

/// A declaration a name refers to: its context, its full name there, and its kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resolved {
    pub context: String,
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    /// A value, entity, aggregate, enum, identity, error, error set or event.
    Named(Resolved),
    /// One of several errors, such as what loading, changing and saving can fail with.
    Errors(Vec<Type>),
    /// What a service holds for a port it uses, such as `keyboard`.
    Port(Resolved),
    /// What a service holds for a repository it uses, such as `rooms`.
    Repository(Resolved),
    /// Not known: after an error has been reported, or the item type of `None`.
    Unknown,
}

impl Type {
    pub fn list(item: Type) -> Type {
        Type::List(Box::new(item))
    }

    pub fn option(item: Type) -> Type {
        Type::Option(Box::new(item))
    }

    pub fn result(value: Type, error: Type) -> Type {
        Type::Result(Box::new(value), Box::new(error))
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Unknown)
    }

    /// The declaration behind a named type, if it is one of `kinds`.
    pub fn named(&self, kinds: &[SymbolKind]) -> Option<&Resolved> {
        match self {
            Type::Named(named) if kinds.contains(&named.kind) => Some(named),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Unit => write!(f, "Unit"),
            Type::List(item) => write!(f, "List<{item}>"),
            Type::Option(item) => write!(f, "Option<{item}>"),
            Type::Result(value, error) => write!(f, "Result<{value}, {error}>"),
            Type::Named(named) | Type::Port(named) | Type::Repository(named) => {
                write!(f, "{}", named.name)
            }
            Type::Errors(errors) => {
                let names: Vec<String> = errors.iter().map(Type::to_string).collect();
                write!(f, "{}", names.join(" or "))
            }
            Type::Unknown => write!(f, "_"),
        }
    }
}
