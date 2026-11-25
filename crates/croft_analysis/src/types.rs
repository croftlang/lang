//! The types a body's expressions can have.

use std::fmt;

use crate::symbols::SymbolKind;

/// Shifts stay below the width of the smallest `Int` on any target, 32 bits.
pub const MAX_SHIFT: i64 = 32;

/// The most decimal places a `Decimal` can have, so its value fits 32 bits.
pub const MAX_PLACES: i64 = 6;

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
    /// A fixed-point number with this many decimal places, kept as a whole
    /// number of hundredths for `Decimal<2>`.
    Decimal(u32),
    Bool,
    String,
    /// A string that holds at most this many characters. Only code
    /// generation makes one, once it has worked out how long a string can
    /// be; the checker never does, so every rule about `String` sees it as one.
    Text(u16),
    /// At most this many characters borrowed from a string that stays put
    /// until the function returns, such as each part of a `split`. Only code
    /// generation makes one.
    Borrowed(u16),
    /// A sequence of bytes, each from 0 to 255: file contents, a save, a
    /// network message.
    Bytes,
    Unit,
    List(Box<Type>),
    /// Keys, each with a value, such as how many of each item the player holds.
    Map(Box<Type>, Box<Type>),
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

    pub fn map(key: Type, value: Type) -> Type {
        Type::Map(Box::new(key), Box::new(value))
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
        matches!(
            self,
            Type::Int | Type::Float | Type::Decimal(_) | Type::Unknown
        )
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
            Type::Decimal(places) => write!(f, "Decimal<{places}>"),
            Type::Bool => write!(f, "Bool"),
            Type::String | Type::Text(_) | Type::Borrowed(_) => write!(f, "String"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Unit => write!(f, "Unit"),
            Type::List(item) => write!(f, "List<{item}>"),
            Type::Map(key, value) => write!(f, "Map<{key}, {value}>"),
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
