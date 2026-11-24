use crate::keyword::Keyword;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Keyword(Keyword),
    Int(i64),
    Float(f64),
    String(String),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    EqualEqual,
    NotEqual,
    Equal,
    Arrow,
    Comma,
    Colon,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    And,
    Or,
    Not,
    Question,
    EndOfFile,
}

impl TokenKind {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        let symbol = match self {
            TokenKind::Identifier(name) => return format!("`{name}`"),
            TokenKind::Keyword(keyword) => return format!("`{}`", keyword.as_str()),
            TokenKind::Int(value) => return format!("`{value}`"),
            TokenKind::Float(value) => return format!("`{value}`"),
            TokenKind::String(_) => return "a string".to_string(),
            TokenKind::EndOfFile => return "the end of the file".to_string(),
            TokenKind::LeftBrace => "{",
            TokenKind::RightBrace => "}",
            TokenKind::LeftParen => "(",
            TokenKind::RightParen => ")",
            TokenKind::LeftBracket => "[",
            TokenKind::RightBracket => "]",
            TokenKind::Less => "<",
            TokenKind::Greater => ">",
            TokenKind::LessEqual => "<=",
            TokenKind::GreaterEqual => ">=",
            TokenKind::EqualEqual => "==",
            TokenKind::NotEqual => "!=",
            TokenKind::Equal => "=",
            TokenKind::Arrow => "->",
            TokenKind::Comma => ",",
            TokenKind::Colon => ":",
            TokenKind::Dot => ".",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::And => "&&",
            TokenKind::Or => "||",
            TokenKind::Not => "!",
            TokenKind::Question => "?",
        };
        format!("`{symbol}`")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// True when a line break comes between this token and the one before.
    pub newline_before: bool,
    /// Comment lines directly above this token, with no blank line between.
    pub doc: Vec<String>,
}
