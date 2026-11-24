use std::collections::HashMap;
use std::fmt;

/// A resolved sp type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Struct(String),
    Enum(String),
    /// A type parameter of a generic constraint, such as `T`.
    Generic(String),
    /// The enum itself used as a value, as in `Currency.Gbp`.
    EnumName(String),
}

impl Type {
    /// Types passed by value rather than by reference in generated code.
    pub fn is_copy(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Float | Type::Bool | Type::Unit | Type::Enum(_)
        )
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Unit => write!(f, "Unit"),
            Type::List(item) => write!(f, "List<{item}>"),
            Type::Option(item) => write!(f, "Option<{item}>"),
            Type::Result(ok, error) => write!(f, "Result<{ok}, {error}>"),
            Type::Struct(name) | Type::Enum(name) | Type::Generic(name) => write!(f, "{name}"),
            Type::EnumName(name) => write!(f, "enum {name}"),
        }
    }
}

/// Matches `actual` against `expected`, binding type parameters on the way.
/// Returns false when the types cannot match.
pub fn unify(expected: &Type, actual: &Type, bindings: &mut HashMap<String, Type>) -> bool {
    match (expected, actual) {
        (Type::Generic(name), _) => match bindings.get(name) {
            Some(bound) => bound.clone() == *actual,
            None => {
                bindings.insert(name.clone(), actual.clone());
                true
            }
        },
        (Type::List(a), Type::List(b)) | (Type::Option(a), Type::Option(b)) => {
            unify(a, b, bindings)
        }
        (Type::Result(a_ok, a_error), Type::Result(b_ok, b_error)) => {
            unify(a_ok, b_ok, bindings) && unify(a_error, b_error, bindings)
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_generic_once() {
        let mut bindings = HashMap::new();
        let list_of_t = Type::List(Box::new(Type::Generic("T".into())));
        assert!(unify(
            &list_of_t,
            &Type::List(Box::new(Type::Int)),
            &mut bindings
        ));
        assert_eq!(bindings["T"], Type::Int);
        assert!(!unify(
            &Type::Generic("T".into()),
            &Type::String,
            &mut bindings
        ));
    }
}
