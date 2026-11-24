use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Int(i64),
    Float(f64),
    String(String),

    // Keywords
    Struct,
    Enum,
    Const,
    Constraint,
    Port,
    Adapter,
    For,
    Fn,
    Is,
    True,
    False,

    // Punctuation
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
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

    EndOfFile,
}

impl TokenKind {
    pub fn keyword(word: &str) -> Option<TokenKind> {
        let kind = match word {
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "const" => TokenKind::Const,
            "constraint" => TokenKind::Constraint,
            "port" => TokenKind::Port,
            "adapter" => TokenKind::Adapter,
            "for" => TokenKind::For,
            "fn" => TokenKind::Fn,
            "is" => TokenKind::Is,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => return None,
        };
        Some(kind)
    }

    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Identifier(name) => format!("identifier `{name}`"),
            TokenKind::Int(value) => format!("integer `{value}`"),
            TokenKind::Float(value) => format!("float `{value}`"),
            TokenKind::String(_) => "string".to_string(),
            TokenKind::EndOfFile => "end of file".to_string(),
            other => format!("`{}`", other.symbol()),
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            TokenKind::Struct => "struct",
            TokenKind::Enum => "enum",
            TokenKind::Const => "const",
            TokenKind::Constraint => "constraint",
            TokenKind::Port => "port",
            TokenKind::Adapter => "adapter",
            TokenKind::For => "for",
            TokenKind::Fn => "fn",
            TokenKind::Is => "is",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::LeftBrace => "{",
            TokenKind::RightBrace => "}",
            TokenKind::LeftParen => "(",
            TokenKind::RightParen => ")",
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
            _ => "?",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
