//! Declarations with behaviour, and the boundary with the outside world.

use super::{Expr, Field, Identifier, Parameter, Path, Rule, TypeExpr};

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
    pub parameters: Option<Vec<Parameter>>,
    pub return_type: Option<TypeExpr>,
    pub return_rules: Vec<Rule>,
    pub preconditions: Vec<Expr>,
    pub uses: Vec<Path>,
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

/// `repository Rooms for Room in Storage`
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub aggregate: Path,
    pub storage: Option<Path>,
}
