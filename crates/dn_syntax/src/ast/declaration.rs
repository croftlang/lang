use super::{
    AdapterDecl, ConstraintUse, Expr, Field, FunctionDecl, Identifier, Parameter, Path, PolicyDecl,
    PortDecl, RepositoryDecl, TypeExpr,
};

/// A context, or the part of one written in a single file.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextDecl {
    pub name: Identifier,
    pub items: Vec<ContextItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextItem {
    Publish(Vec<Path>),
    Use(UseDecl),
    Value(ValueDecl),
    Enum(EnumDecl),
    Error(RecordDecl),
    Const(ConstDecl),
    Identity(IdentityDecl),
    Constraint(ConstraintDecl),
    Aggregate(AggregateDecl),
    Function(FunctionDecl),
    Policy(PolicyDecl),
    Port(PortDecl),
    Adapter(AdapterDecl),
    Repository(RepositoryDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub names: Vec<Path>,
    pub from: Identifier,
}

/// A value or an entity: named fields with optional invariants.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub fields: Vec<Field>,
    pub invariants: Vec<Invariant>,
}

/// An aggregate, or the part of one written in a single file.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub fields: Vec<Field>,
    pub invariants: Vec<Invariant>,
    pub entities: Vec<ValueDecl>,
    pub errors: Vec<RecordDecl>,
    pub events: Vec<RecordDecl>,
    pub commands: Vec<FunctionDecl>,
}

/// `invariant ExitClear or ExitBlocked`
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    pub constraint: ConstraintUse,
    pub error: Option<Expr>,
}

/// An error, event or enum variant: a name with optional fields.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub fields: Vec<Field>,
}

/// An enum. `include Room.Error` adds every variant of another enum.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub includes: Vec<Path>,
    pub variants: Vec<RecordDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub ty: TypeExpr,
    pub value: Expr,
}

/// `identity RoomId: Int where Range(1, 20)`
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub ty: TypeExpr,
    pub constraints: Vec<ConstraintUse>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub parameters: Vec<Parameter>,
    pub body: Expr,
}

/// `domain Game { ... } = { ... }`: the whole program. It lists the contexts
/// the domain is divided into, uses what they publish, and its body runs first.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    /// Every context named in `contexts`, once for each time it is written.
    pub contexts: Vec<Identifier>,
    /// The context map: `Dungeon -> Scoring` means Scoring is downstream of
    /// Dungeon, so Scoring may use what Dungeon publishes.
    pub relations: Vec<(Identifier, Identifier)>,
    /// What it imports from contexts with `use { ... } from`.
    pub uses: Vec<UseDecl>,
    /// The ports the body calls, as a service lists them.
    pub calls: Vec<Path>,
    pub body: Expr,
}
