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
    /// A number in place of type arguments, such as the places in `Decimal<2>`.
    pub number: Option<i64>,
}

impl TypeExpr {
    /// The type as written, with one space after each comma: `Map<String, Int>`.
    pub fn text(&self) -> String {
        let mut text = self.path.text();

        if let Some(number) = self.number {
            text.push_str(&format!("<{number}>"));
        } else if !self.arguments.is_empty() {
            let arguments: Vec<String> = self.arguments.iter().map(TypeExpr::text).collect();
            text.push_str(&format!("<{}>", arguments.join(", ")));
        }

        text
    }
}

/// A constraint applied to a value after `where`: `Range(0, 100)`, or with
/// `each`, applied to every item of a list. The value is its first argument.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintUse {
    pub each: bool,
    pub name: Path,
    pub arguments: Vec<Expr>,
}

impl ConstraintUse {
    /// The constraint as written, for messages: `Range(1, 50)` or `each Alive`.
    pub fn text(&self) -> String {
        let name = self.name.text();
        let arguments: Vec<String> = self.arguments.iter().map(argument_text).collect();

        let call = if arguments.is_empty() {
            name
        } else {
            format!("{name}({})", arguments.join(", "))
        };

        if self.each {
            format!("each {call}")
        } else {
            call
        }
    }
}

/// A constraint's argument as written, such as `MAX_ENEMIES - 1`.
fn argument_text(value: &Expr) -> String {
    match &value.kind {
        ExprKind::Int(number) => number.to_string(),
        ExprKind::Float(text) => text.clone(),
        ExprKind::Name(name) => name.clone(),
        ExprKind::Unary(UnaryOp::Negate, inner) => format!("-{}", argument_text(inner)),
        ExprKind::Unary(UnaryOp::BitNot, inner) => format!("~{}", argument_text(inner)),
        ExprKind::Binary(op, left, right) => {
            let symbol = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
                BinaryOp::BitAnd => "&",
                BinaryOp::BitOr => "|",
                BinaryOp::BitXor => "^",
                BinaryOp::ShiftLeft => "<<",
                BinaryOp::ShiftRight => ">>",
                _ => "?",
            };

            format!("{} {symbol} {}", argument_text(left), argument_text(right))
        }
        _ => "...".to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub doc: Vec<String>,
    pub name: Identifier,
    pub ty: TypeExpr,
    pub constraints: Vec<ConstraintUse>,
    /// Whether this field holds its aggregate's or entity's own identity: it is
    /// of the identity type declared `for` them. Set by analysis, not written.
    pub is_identity: bool,
    /// The starting value of an adapter's field: `keys: List<Key> = []`.
    pub value: Option<Expr>,
}

/// A parameter. Adapter operations leave out the type, which comes from the port.
/// A constraint parameter may accept several types: `items: String or List<T>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Identifier,
    pub types: Vec<TypeExpr>,
    pub constraints: Vec<ConstraintUse>,
}

/// Everything read from one source file: the contexts, or parts of
/// contexts, it declares.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct File {
    pub contexts: Vec<ContextDecl>,
    pub domains: Vec<DomainDecl>,
}
