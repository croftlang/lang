//! Expressions, statements and patterns.

use super::{ConstraintUse, Identifier, Path, TypeExpr};
use crate::span::{SourceId, Span};

/// An expression, where it was written, and its id.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    /// Tells this expression apart from every other in the program, so later
    /// passes can record facts about it, such as its type.
    pub id: ExprId,
}

/// The id of an expression: its source file and the order the parser made it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId {
    pub source: SourceId,
    pub index: u32,
}

/// What kind of expression it is.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    /// As written, such as `2.50`.
    Float(String),
    /// `["sword": 3, "key": 1]`, or `[:]` when empty.
    Map(Vec<(Expr, Expr)>),
    String(String),
    Bool(bool),
    Name(String),
    Field(Box<Expr>, Identifier),
    Call(Box<Expr>, Vec<Argument>),
    /// `Position { x: 10, y: 20 }`
    Record(Path, Vec<FieldValue>),
    /// `start with { x: 11 }`
    With(Box<Expr>, Vec<FieldValue>),
    /// `[a, b, c]`, or `[]` for an empty list.
    List(Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// `value or Error`: the error to fail with when `value` fails.
    OrElse(Box<Expr>, Box<Expr>),
    /// `value?`: pass a failure up.
    Try(Box<Expr>),
    Lambda(Vec<Identifier>, Box<Expr>),
    /// A guard: when the condition holds, the block is the result.
    If(Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Vec<MatchArm>),
    /// `loop { ... }`, or `loop (let i where Range(0, 99)) { ... }` with a counter.
    Loop(Option<LoopCounter>, Box<Expr>),
    Break,
    Block(Vec<Statement>),
    Issue(Issue),
    Emit(Path, Vec<FieldValue>),
    Native(Identifier, Vec<Argument>),
}

/// `issue Attack(enemy_id: target, damage: 5) on room`
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub command: Identifier,
    pub arguments: Vec<Argument>,
    pub target: Box<Expr>,
}

/// A call argument, named (`amount: 100`) or positional.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub name: Option<Identifier>,
    pub value: Expr,
}

/// `x: 10`, or just `enemy_id` as shorthand for `enemy_id: enemy_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldValue {
    pub name: Identifier,
    pub value: Option<Expr>,
}

/// The counter of a counted loop: it takes every value its constraints allow, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopCounter {
    pub name: Identifier,
    pub constraints: Vec<ConstraintUse>,
}

/// One step of a code block.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `let name = value`, or `let name: Type = value`.
    Let(Identifier, Option<TypeExpr>, Expr),
    Set(Identifier, Expr),
    Expr(Expr),
}

/// One arm of a `match`: a pattern and the value it gives back.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub value: Expr,
}

/// `Coins(count)`, `Item.Gold(amount)` or `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub path: Path,
    pub bindings: Vec<Identifier>,
}

/// An operator written before its one operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
    /// `~bits`: every bit flipped.
    BitNot,
}

/// An operator written between its two operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}
