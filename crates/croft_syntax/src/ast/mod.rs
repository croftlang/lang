//! The syntax tree produced by the parser.

mod declaration;
mod expression;
mod function;

pub use declaration::*;
pub use expression::*;
pub use function::*;

use crate::span::Span;

/// A name as written, and where.
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
    /// Where the path starts.
    pub fn span(&self) -> Span {
        self.segments[0].span
    }

    /// The last segment: `EnemyDefeated` in `Room.EnemyDefeated`.
    pub fn last(&self) -> &Identifier {
        self.segments
            .last()
            .expect("a path has at least one segment")
    }

    /// The path as written, its segments joined with `.`.
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
    /// `!numbers.is_empty`. Blocks, matches and the like are `...`. The
    /// tree keeps no brackets, so they are put back wherever the operators
    /// need them: `n % (2 - 2)`.
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
            ExprKind::Field(receiver, field) => {
                format!("{}.{}", receiver.operand_text(), field.name)
            }
            ExprKind::Call(callee, arguments) => {
                let arguments: Vec<String> = (arguments.iter())
                    .map(|argument| match &argument.name {
                        Some(name) => format!("{}: {}", name.name, argument.value.text()),
                        None => argument.value.text(),
                    })
                    .collect();

                format!("{}({})", callee.operand_text(), arguments.join(", "))
            }
            ExprKind::List(items) => format!("[{}]", list(&mut items.iter())),
            ExprKind::Unary(op, inner) => format!("{}{}", op.symbol(), inner.unary_operand_text()),
            ExprKind::Binary(op, left, right) => {
                let left = left.binary_operand_text(op.precedence());
                let right = right.binary_operand_text(op.precedence() + 1);

                format!("{left} {} {right}", op.symbol())
            }
            _ => "...".to_string(),
        }
    }

    /// The text of an expression before `.` or `(`, bracketed when it is an
    /// operator: `(a + b).abs()`.
    fn operand_text(&self) -> String {
        match &self.kind {
            ExprKind::Unary(..) | ExprKind::Binary(..) => format!("({})", self.text()),
            _ => self.text(),
        }
    }

    /// The text of an expression after `!`, `-` or `~`, bracketed when it is
    /// a binary operator: `-(a + b)`.
    fn unary_operand_text(&self) -> String {
        match &self.kind {
            ExprKind::Binary(..) => format!("({})", self.text()),
            _ => self.text(),
        }
    }

    /// The text of one side of a binary operator, bracketed when its own
    /// operator binds less tightly than `min_precedence`. The right side asks
    /// for one more than the operator's own, since operators group from the
    /// left: `a - (b - c)`.
    fn binary_operand_text(&self, min_precedence: u8) -> String {
        match &self.kind {
            ExprKind::Binary(op, ..) if op.precedence() < min_precedence => {
                format!("({})", self.text())
            }
            _ => self.text(),
        }
    }
}

impl UnaryOp {
    /// The operator as written: `!`, `-` or `~`.
    pub fn symbol(&self) -> &'static str {
        match self {
            UnaryOp::Not => "!",
            UnaryOp::Negate => "-",
            UnaryOp::BitNot => "~",
        }
    }
}

impl BinaryOp {
    /// How tightly the operator binds: higher binds tighter. Bit operators bind
    /// tighter than comparisons, so `bits & FIRE != 0` means `(bits & FIRE) != 0`,
    /// unlike in C.
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Equal | BinaryOp::NotEqual => 3,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 4,
            BinaryOp::BitOr => 5,
            BinaryOp::BitXor => 6,
            BinaryOp::BitAnd => 7,
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight => 8,
            BinaryOp::Add | BinaryOp::Subtract => 9,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 10,
        }
    }

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

/// A field of a value, entity, aggregate, error, event, enum variant or adapter.
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
