//! Laying tokens and comments out in lines: which tokens share a line,
//! how far each line is indented, and where blank lines go.

use croft_syntax::keyword::Keyword;
use croft_syntax::lexer::TokenizedSource;
use croft_syntax::token::{Token, TokenKind};

use crate::roles::{find_roles, Role};
use crate::spacing::space_between;

/// One level of indentation.
const INDENT: &str = "    ";

/// The tokens and comment that start on one line of the source, or on
/// lines held together by a string written across them.
struct SourceLine {
    /// The first token on the line and the one after the last, by index.
    tokens: std::ops::Range<usize>,
    /// The comment that ends the line, by index.
    comment: Option<usize>,
    /// Whether a blank line came before it.
    blank_before: bool,
}

/// A bracket that has been opened and not yet closed.
struct OpenBracket {
    kind: TokenKind,
    /// The indentation of the line it was opened on.
    indent: usize,
}

/// Writes the file out again, laid out.
pub(crate) fn lay_out(source: &str, tokenized: &TokenizedSource) -> String {
    let tokens = &tokenized.tokens;
    let roles = find_roles(tokens);
    let texts: Vec<&str> = tokenized
        .ranges
        .iter()
        .map(|range| &source[range.clone()])
        .collect();
    let lines = group_lines(tokenized, &texts);

    let mut output = String::new();
    let mut open: Vec<OpenBracket> = Vec::new();
    let mut last_code: Option<usize> = None;
    let mut previous_opens_block = false;

    for line in &lines {
        let first = (!line.tokens.is_empty()).then_some(line.tokens.start);
        let starts_with_closer = first.is_some_and(|index| is_closer(&tokens[index].kind));
        let content_indent = open.last().map_or(0, |bracket| bracket.indent + 1);

        let indent = match first {
            Some(_) if starts_with_closer => open.last().map_or(0, |bracket| bracket.indent),
            Some(index) if continues_expression(tokens, &roles, last_code, index) => {
                content_indent + 1
            }
            _ => content_indent,
        };

        let keeps_blank = line.blank_before && !previous_opens_block && !starts_with_closer;

        if keeps_blank && !output.is_empty() {
            output.push('\n');
        }

        output.push_str(&INDENT.repeat(indent));

        let open_at_start = open.len();

        for index in line.tokens.clone() {
            if index > line.tokens.start {
                // Only a bracket opened on this line holds fields written side by side.
                let inside = open
                    .last()
                    .filter(|_| open.len() > open_at_start)
                    .map(|bracket| &bracket.kind);

                output.push_str(space_between(tokens, &roles, index - 1, index, inside));
            }

            output.push_str(texts[index]);

            if is_opener(&tokens[index].kind) {
                open.push(OpenBracket {
                    kind: tokens[index].kind.clone(),
                    indent,
                });
            } else if is_closer(&tokens[index].kind) {
                open.pop();
            }
        }

        if let Some(comment) = line.comment {
            if !line.tokens.is_empty() {
                output.push(' ');
            }

            output.push_str("//");
            output.push_str(&tokenized.comments[comment].text);
        }

        output.push('\n');

        if let Some(last) = line.tokens.clone().last() {
            last_code = Some(last);
        }

        // A blank line just inside a bracket is dropped, unless a comment
        // ends the line, since the comment would then become a doc comment.
        previous_opens_block = line.comment.is_none()
            && line
                .tokens
                .clone()
                .last()
                .is_some_and(|last| is_opener(&tokens[last].kind));
    }

    output
}

/// Sorts the tokens and comments into lines, in the order written.
fn group_lines(tokenized: &TokenizedSource, texts: &[&str]) -> Vec<SourceLine> {
    let tokens = &tokenized.tokens;
    let comments = &tokenized.comments;
    let code_tokens = tokens.len().saturating_sub(1);

    let mut lines: Vec<SourceLine> = Vec::new();
    let mut last_line = 0;
    let (mut token, mut comment) = (0, 0);

    while token < code_tokens || comment < comments.len() {
        let token_first = comment == comments.len()
            || (token < code_tokens
                && position(&tokens[token])
                    < (comments[comment].span.line, comments[comment].span.column));

        let start_line = if token_first {
            tokens[token].span.line
        } else {
            comments[comment].span.line
        };

        if lines.is_empty() || start_line > last_line {
            lines.push(SourceLine {
                tokens: token..token,
                comment: None,
                blank_before: !lines.is_empty() && start_line > last_line + 1,
            });
        }

        let line = lines.last_mut().expect("a line was just added");

        if token_first {
            line.tokens.end = token + 1;

            let breaks = texts[token].matches('\n').count() as u32;
            last_line = start_line + breaks;
            token += 1;
            continue;
        }

        line.comment = Some(comment);
        last_line = start_line;
        comment += 1;
    }

    lines
}

/// Where the token starts, as line and column.
fn position(token: &Token) -> (u32, u32) {
    (token.span.line, token.span.column)
}

/// Whether the token is `{`, `(` or `[`.
fn is_opener(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket
    )
}

/// Whether the token is `}`, `)` or `]`.
fn is_closer(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket
    )
}

/// Whether the line starting with the token at `first` carries on the
/// expression above it: it starts with `.` or `?`, or the line above ended
/// with an operator, `=` or `->`. Such a line is indented one level more.
fn continues_expression(
    tokens: &[Token],
    roles: &[Role],
    last_code: Option<usize>,
    first: usize,
) -> bool {
    if matches!(tokens[first].kind, TokenKind::Dot | TokenKind::Question) {
        return true;
    }

    let Some(last) = last_code else {
        return false;
    };

    if roles[last] == Role::TypeBracket {
        return false;
    }

    matches!(
        tokens[last].kind,
        TokenKind::Equal
            | TokenKind::Arrow
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::EqualEqual
            | TokenKind::NotEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Ampersand
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::Keyword(Keyword::Or)
    )
}
