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
    /// The error after `or`: `where Overdose(4000) or Overdosed`. A broken
    /// constraint with one fails the call with it; one without is a mistake.
    pub error: Option<Expr>,
}

impl ConstraintUse {
    /// The constraint as written, for messages: `Range(1, 50)` or `each Alive`.
    pub fn text(&self) -> String {
        let name = self.name.text();
        let arguments: Vec<String> = self.arguments.iter().map(Expr::text).collect();

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

impl Expr {
    /// The expression as written, for messages: `MAX_ENEMIES - 1` or
    /// `!numbers.is_empty`. Blocks, matches and the like are `...`.
    pub fn text(&self) -> String {
        let list = |values: &mut dyn Iterator<Item = &Expr>| {
            values.map(Expr::text).collect::<Vec<_>>().join(", ")
        };

        match &self.kind {
            ExprKind::Int(number) => number.to_string(),
            ExprKind::Float(text) => text.clone(),
            ExprKind::String(text) => format!("{text:?}"),
            ExprKind::Bool(value) => value.to_string(),
            ExprKind::Name(name) => name.clone(),
            ExprKind::Field(receiver, field) => format!("{}.{}", receiver.text(), field.name),
            ExprKind::Call(callee, arguments) => {
                let arguments: Vec<String> = (arguments.iter())
                    .map(|argument| match &argument.name {
                        Some(name) => format!("{}: {}", name.name, argument.value.text()),
                        None => argument.value.text(),
                    })
                    .collect();

                format!("{}({})", callee.text(), arguments.join(", "))
            }
            ExprKind::List(items) => format!("[{}]", list(&mut items.iter())),
            ExprKind::Unary(UnaryOp::Not, inner) => format!("!{}", inner.text()),
            ExprKind::Unary(UnaryOp::Negate, inner) => format!("-{}", inner.text()),
            ExprKind::Unary(UnaryOp::BitNot, inner) => format!("~{}", inner.text()),
            ExprKind::Binary(op, left, right) => {
                format!("{} {} {}", left.text(), op.symbol(), right.text())
            }
            _ => "...".to_string(),
        }
    }
}

impl BinaryOp {
    /// The operator as written: `+`, `<=`, `&&`.
    pub fn symbol(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Remainder => "%",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::ShiftLeft => "<<",
            BinaryOp::ShiftRight => ">>",
        }
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
