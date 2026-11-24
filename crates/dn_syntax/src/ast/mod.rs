//! The syntax tree produced by the parser.

mod declaration;
mod expression;
mod function;

pub use declaration::*;
pub use expression::*;
pub use function::*;

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

/// A possibly dotted name, such as `Room` or `Dungeon.Room.EnemyDefeated`.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<Identifier>,
}

impl Path {
    pub fn span(&self) -> Span {
        self.segments[0].span
    }

    pub fn last(&self) -> &Identifier {
        self.segments
            .last()
            .expect("a path has at least one segment")
    }

    pub fn text(&self) -> String {
        let names: Vec<&str> = self.segments.iter().map(|s| s.name.as_str()).collect();
        names.join(".")
    }
}

/// A type as written, such as `Int` or `List<Room.Enemy>`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub path: Path,
    pub arguments: Vec<TypeExpr>,
}

/// One constraint after `where`, applied to the value or, with `each`, to every item.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub each: bool,
    pub constraint: ConstraintUse,
}

/// A constraint applied to a value: `Range(0, 100)`. The value is its first argument.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintUse {
    pub name: Path,
    pub arguments: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub ty: TypeExpr,
    pub rules: Vec<Rule>,
    pub is_identity: bool,
}

/// A parameter. Adapter operations leave out the type, which comes from the port.
/// A constraint parameter may accept several types: `items: String or List<T>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Identifier,
    pub types: Vec<TypeExpr>,
    pub rules: Vec<Rule>,
}

/// Everything read from one source file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct File {
    pub items: Vec<TopItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopItem {
    Context(ContextDecl),
    Application(ApplicationDecl),
}
