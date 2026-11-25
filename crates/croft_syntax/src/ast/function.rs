//! Declarations with behaviour, and the boundary with the outside world.

use super::{ConstraintUse, Expr, Field, Identifier, Parameter, Path, TypeExpr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Function,
    Service,
    Translation,
    Command,
    Operation,
    /// The body of a `domain`, which runs first.
    Domain,
}

/// Anything with a signature: functions, services, translations, commands and
/// port or adapter operations. Commands without parameters have none.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub doc: Vec<String>,
    pub kind: FunctionKind,
    pub name: Identifier,
    /// `T` in `fn first<T>(items: List<T>): Option<T>`.
    pub type_parameters: Vec<Identifier>,
    pub parameters: Option<Vec<Parameter>>,
    pub return_type: Option<TypeExpr>,
    pub return_constraints: Vec<ConstraintUse>,
    pub preconditions: Vec<Expr>,
    /// The ports a service or domain calls: `calls Screen, Keyboard`.
    pub calls: Vec<Path>,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub event: Path,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub operations: Vec<FunctionDecl>,
}

/// `adapter SpectrumScreen for Screen = { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub port: Path,
    pub fields: Vec<Field>,
    pub operations: Vec<FunctionDecl>,
}

/// `repository Rooms for Room`. Where it saves is chosen in croft.toml.
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub aggregate: Path,
}
