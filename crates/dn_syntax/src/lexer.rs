use crate::span::{Diagnostic, SourceId, Span};
use crate::token::{Token, TokenKind};

/// Turns source text into tokens. Whitespace and `//` comments are skipped.
pub fn tokenize(source: &str, source_id: SourceId) -> Result<Vec<Token>, Diagnostic> {
    Lexer::new(source, source_id).run()
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    source_id: SourceId,
    line: u32,
    column: u32,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, source_id: SourceId) -> Self {
        Self {
            chars: source.chars().peekable(),
            source_id,
            line: 1,
            column: 1,
        }
    }

    fn span(&self) -> Span {
        Span {
            source: self.source_id,
            line: self.line,
            column: self.column,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let next = self.chars.next()?;
        if next == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(next)
    }

    fn advance_if(&mut self, expected: char) -> bool {
        if self.chars.peek() == Some(&expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn run(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let span = self.span();
            let Some(current) = self.advance() else {
                tokens.push(Token {
                    kind: TokenKind::EndOfFile,
                    span,
                });
                return Ok(tokens);
            };
            let kind = match current {
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                '.' => TokenKind::Dot,
                '+' => TokenKind::Plus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '-' if self.advance_if('>') => TokenKind::Arrow,
                '-' => TokenKind::Minus,
                '<' if self.advance_if('=') => TokenKind::LessEqual,
                '<' => TokenKind::Less,
                '>' if self.advance_if('=') => TokenKind::GreaterEqual,
                '>' => TokenKind::Greater,
                '=' if self.advance_if('=') => TokenKind::EqualEqual,
                '=' => TokenKind::Equal,
                '!' if self.advance_if('=') => TokenKind::NotEqual,
                '!' => TokenKind::Not,
                '&' if self.advance_if('&') => TokenKind::And,
                '|' if self.advance_if('|') => TokenKind::Or,
                '"' => self.string(span)?,
                c if c.is_ascii_digit() => self.number(c, span)?,
                c if c.is_alphabetic() || c == '_' => self.word(c),
                other => {
                    return Err(Diagnostic::new(
                        format!("unexpected character `{other}`"),
                        span,
                    ))
                }
            };
            tokens.push(Token { kind, span });
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.chars.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') => {
                    let mut lookahead = self.chars.clone();
                    lookahead.next();
                    if lookahead.peek() != Some(&'/') {
                        return;
                    }
                    while let Some(c) = self.chars.peek() {
                        if *c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self, start: Span) -> Result<TokenKind, Diagnostic> {
        let mut value = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(TokenKind::String(value)),
                Some('\\') => match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(other) => {
                        return Err(Diagnostic::new(
                            format!("unknown escape `\\{other}`"),
                            self.span(),
                        ))
                    }
                    None => break,
                },
                Some(c) => value.push(c),
                None => break,
            }
        }
        Err(Diagnostic::new("unterminated string", start))
    }

    fn number(&mut self, first: char, start: Span) -> Result<TokenKind, Diagnostic> {
        let mut text = first.to_string();
        let mut is_float = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '_' {
                self.advance();
                if c != '_' {
                    text.push(c);
                }
            } else if c == '.' && !is_float {
                // Only treat `.` as a decimal point when a digit follows,
                // so `list.length` style access still works after numbers.
                let mut lookahead = self.chars.clone();
                lookahead.next();
                match lookahead.peek() {
                    Some(d) if d.is_ascii_digit() => {
                        is_float = true;
                        self.advance();
                        text.push('.');
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
        if is_float {
            text.parse()
                .map(TokenKind::Float)
                .map_err(|_| Diagnostic::new(format!("invalid float `{text}`"), start))
        } else {
            text.parse()
                .map(TokenKind::Int)
                .map_err(|_| Diagnostic::new(format!("integer `{text}` is too large"), start))
        }
    }

    fn word(&mut self, first: char) -> TokenKind {
        let mut text = first.to_string();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
                text.push(c);
            } else {
                break;
            }
        }
        TokenKind::keyword(&text).unwrap_or(TokenKind::Identifier(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source, SourceId(0))
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn reads_keywords_and_punctuation() {
        assert_eq!(
            kinds("struct Money { amount: Int } // comment"),
            vec![
                TokenKind::Struct,
                TokenKind::Identifier("Money".into()),
                TokenKind::LeftBrace,
                TokenKind::Identifier("amount".into()),
                TokenKind::Colon,
                TokenKind::Identifier("Int".into()),
                TokenKind::RightBrace,
                TokenKind::EndOfFile,
            ]
        );
    }

    #[test]
    fn reads_numbers_and_operators() {
        assert_eq!(
            kinds("1_000 >= 2.5 -> !="),
            vec![
                TokenKind::Int(1000),
                TokenKind::GreaterEqual,
                TokenKind::Float(2.5),
                TokenKind::Arrow,
                TokenKind::NotEqual,
                TokenKind::EndOfFile,
            ]
        );
    }

    #[test]
    fn tracks_line_and_column() {
        let tokens = tokenize("a\n  b", SourceId(0)).unwrap();
        assert_eq!((tokens[1].span.line, tokens[1].span.column), (2, 3));
    }
}
