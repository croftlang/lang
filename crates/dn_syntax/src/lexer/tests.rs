use super::tokenize;
use crate::keyword::Keyword;
use crate::span::SourceId;
use crate::token::{Token, TokenKind};

fn tokens(source: &str) -> Vec<Token> {
    tokenize(source, SourceId(0)).expect("source should tokenize")
}

fn token_kinds(source: &str) -> Vec<TokenKind> {
    tokens(source).into_iter().map(|token| token.kind).collect()
}

#[test]
fn reads_keywords_and_identifiers() {
    assert_eq!(
        token_kinds("aggregate Room on"),
        vec![
            TokenKind::Keyword(Keyword::Aggregate),
            TokenKind::Identifier("Room".into()),
            TokenKind::Identifier("on".into()),
            TokenKind::EndOfFile,
        ]
    );
}

#[test]
fn reads_numbers() {
    assert_eq!(
        token_kinds("999_999 0x4F 2.5 list.length"),
        vec![
            TokenKind::Int(999_999),
            TokenKind::Int(0x4F),
            TokenKind::Float(2.5),
            TokenKind::Identifier("list".into()),
            TokenKind::Dot,
            TokenKind::Identifier("length".into()),
            TokenKind::EndOfFile,
        ]
    );
}

#[test]
fn reads_operators() {
    assert_eq!(
        token_kinds("-> != ? <="),
        vec![
            TokenKind::Arrow,
            TokenKind::NotEqual,
            TokenKind::Question,
            TokenKind::LessEqual,
            TokenKind::EndOfFile,
        ]
    );
}

#[test]
fn records_line_breaks() {
    let found = tokens("a b\nc");
    assert!(!found[1].newline_before);
    assert!(found[2].newline_before);
}

#[test]
fn attaches_comments_directly_above_as_doc() {
    let found = tokens("// first\n// second\ncommand Reset");
    assert_eq!(found[0].doc, vec!["first", "second"]);
}

#[test]
fn ignores_comments_separated_by_a_blank_line() {
    let found = tokens("// note\n\ncommand Reset");
    assert!(found[0].doc.is_empty());
}

#[test]
fn reports_unterminated_strings() {
    let error = tokenize("\"open", SourceId(0)).unwrap_err();
    assert!(error.message.contains("unterminated"));
}

#[test]
fn reads_binary_numbers_and_bit_operators() {
    assert_eq!(
        token_kinds("0b1010_0001 a & b | c ^ ~d << 2 >> 1"),
        vec![
            TokenKind::Int(0b1010_0001),
            TokenKind::Identifier("a".to_string()),
            TokenKind::Ampersand,
            TokenKind::Identifier("b".to_string()),
            TokenKind::Pipe,
            TokenKind::Identifier("c".to_string()),
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::Identifier("d".to_string()),
            TokenKind::ShiftLeft,
            TokenKind::Int(2),
            TokenKind::ShiftRight,
            TokenKind::Int(1),
            TokenKind::EndOfFile,
        ]
    );
}
