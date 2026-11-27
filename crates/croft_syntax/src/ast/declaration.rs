//! Declarations at context level, and the domain.

use super::{
    AdapterDecl, ConstraintUse, Expr, Field, FunctionDecl, Identifier, Parameter, Path, PolicyDecl,
    PortDecl, RepositoryDecl, TypeExpr,
};

/// A context, or the part of one written in a single file.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub items: Vec<ContextItem>,
}

/// One declaration inside a context.
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

/// `use { Edge, Path } from Routing`: names imported from another context.
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub names: Vec<ImportedName>,
    /// A context, or a standard context such as `Std.Text`.
    pub from: Path,
}

/// A name brought in by `use` or `calls`: `Keyboard`, or `Keyboard as keyboard`
/// to call it something else here.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedName {
    /// The name as the other context declares it.
    pub path: Path,
    /// The name after `as`, which replaces the declared one.
    pub alias: Option<Identifier>,
}

impl ImportedName {
    /// The name it goes by here: the alias, or the declared name.
    pub fn local_name(&self) -> String {
        match &self.alias {
            Some(alias) => alias.name.clone(),
            None => self.path.text(),
        }
    }

    /// As written: `Keyboard`, or `Keyboard as keyboard`.
    pub fn text(&self) -> String {
        match &self.alias {
            Some(alias) => format!("{} as {}", self.path.text(), alias.name),
            None => self.path.text(),
        }
    }
}

/// A port or repository a service or domain may call, from `calls { ... }`,
/// with the context it comes from when that is written after `from`.
#[derive(Debug, Clone, PartialEq)]
pub struct CalledPort {
    pub name: ImportedName,
    /// A context, or a standard context such as `Std.Http`. Left out for a
    /// port the code can already see.
    pub from: Option<Path>,
}

/// A value or an entity: named fields with optional invariants.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    /// `A` and `B` in `value Pair<A, B> { first: A  second: B }`.
    pub type_parameters: Vec<Identifier>,
    pub fields: Vec<Field>,
    pub invariants: Vec<Invariant>,
    /// Written `value Score: Int where Range(0, 999_999)`: a type of its own
    /// around one value, held in a single field called `value`.
    pub single: bool,
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
    /// The only ways to make a new one: `factory Book(...) = Cargo { ... }`.
    pub factories: Vec<FunctionDecl>,
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

/// `const MAX_ENEMIES: Int = 16`
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub ty: TypeExpr,
    pub value: Expr,
}

/// `identity RoomId for Room: Int where Range(1, 20)`
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityDecl {
    pub doc: Vec<String>,
    pub name: Identifier,
    /// The aggregate or entity it identifies: `Room`, or `Room.Enemy`.
    pub owner: Path,
    pub ty: TypeExpr,
    pub constraints: Vec<ConstraintUse>,
}

/// `constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0`
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
    pub calls: Vec<CalledPort>,
    pub body: Expr,
}
