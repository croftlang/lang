//! Generics. `value Pair<A, B>` and `fn first<T>` are templates: they are
//! checked and compiled only as instances, such as `Pair<Int, String>` or
//! `first<RoomId>`, made where they are used. Making an instance can lead to
//! another, so checking runs in rounds until no new instance is asked for.
//! Each instance is a copy of its template with the type arguments written
//! in, added to the context that uses it, so it is checked and compiled like
//! anything written by hand.

mod instantiate;
mod renumber;

pub use instantiate::instance_file;

use b_syntax::ast::TypeExpr;

/// An instance a context needs: the template, where it is declared, and
/// the types written in for its type parameters.
#[derive(Debug, Clone)]
pub struct Request {
    /// The context the instance is made in: the one that uses it.
    pub context: String,
    pub template_context: String,
    pub template: String,
    pub arguments: Vec<TypeExpr>,
}

/// Two requests are the same when they make the same instance in the same
/// place, however the type arguments were written.
impl PartialEq for Request {
    fn eq(&self, other: &Request) -> bool {
        self.context == other.context
            && self.template_context == other.template_context
            && self.name() == other.name()
    }
}

impl Request {
    /// The instance's name, such as `Pair<Int, String>`.
    pub fn name(&self) -> String {
        instance_name(&self.template, &self.arguments)
    }
}

pub fn instance_name(template: &str, arguments: &[TypeExpr]) -> String {
    let arguments: Vec<String> = arguments.iter().map(TypeExpr::text).collect();
    format!("{template}<{}>", arguments.join(", "))
}

/// Whether a type parameter's name is a single capital letter, such as `T`.
pub fn is_type_parameter_name(name: &str) -> bool {
    name.len() == 1 && name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// How many rounds of making instances there can be before the program is
/// taken to ask for them without end, as `Box<T>` holding a `Box<Box<T>>` would.
pub const MAX_ROUNDS: usize = 8;
