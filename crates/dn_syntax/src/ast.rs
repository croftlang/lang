//! The syntax tree produced by the parser.

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

/// All items read from one source file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    Constraint(ConstraintDecl),
    Port(PortDecl),
    Adapter(AdapterDecl),
    Function(FunctionDecl),
}

impl Item {
    pub fn name(&self) -> &Identifier {
        match self {
            Item::Struct(item) => &item.name,
            Item::Enum(item) => &item.name,
            Item::Const(item) => &item.name,
            Item::Constraint(item) => &item.name,
            Item::Port(item) => &item.name,
            Item::Adapter(item) => &item.name,
            Item::Function(item) => &item.signature.name,
        }
    }
}

/// A type as written in source, such as `Int` or `Result<Invoice, NotFound>`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub name: Identifier,
    pub arguments: Vec<TypeExpr>,
}

/// `struct Invoice { lines: List<Line> is NonEmpty } is DueAfterIssue`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: Identifier,
    pub fields: Vec<Field>,
    pub constraints: Vec<ConstraintUse>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: Identifier,
    pub ty: TypeExpr,
    pub constraints: Vec<ConstraintUse>,
}

/// A constraint applied to a field or struct, such as `MaxLength(100)`.
/// The field or struct is passed as the first parameter; `arguments` fill the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintUse {
    pub name: Identifier,
    pub arguments: Vec<Expr>,
}

/// `enum Currency { Gbp, Usd, Eur }`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Identifier,
    pub variants: Vec<Identifier>,
}

/// `const MAX_LINES: Int = 100`
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: Identifier,
    pub ty: TypeExpr,
    pub value: Expr,
}

/// `constraint NonEmpty<T>(list: List<T>) = list.length > 0`
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub name: Identifier,
    pub generics: Vec<Identifier>,
    pub parameters: Vec<Parameter>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Identifier,
    pub ty: TypeExpr,
}

/// `port InvoiceRepository { fn find(id: InvoiceId) -> Result<Invoice, NotFound> }`
#[derive(Debug, Clone, PartialEq)]
pub struct PortDecl {
    pub name: Identifier,
    pub operations: Vec<Signature>,
}

/// `adapter InMemoryInvoices for InvoiceRepository`
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterDecl {
    pub name: Identifier,
    pub port: Identifier,
}

/// A name, parameters and output type, shared by functions and port operations.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    pub name: Identifier,
    pub parameters: Vec<Parameter>,
    pub output: TypeExpr,
}

/// A function: its signature and contract. The body is not written in dn.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub signature: Signature,
    pub uses: Vec<Identifier>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Name(String),
    Field(Box<Expr>, Identifier),
    Call(Identifier, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
}

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
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
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
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
        )
    }

    pub fn is_equality(self) -> bool {
        matches!(self, BinaryOp::Equal | BinaryOp::NotEqual)
    }

    pub fn is_logical(self) -> bool {
        matches!(self, BinaryOp::And | BinaryOp::Or)
    }
}
