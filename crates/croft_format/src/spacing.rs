//! The space between two tokens on the same line.

use croft_syntax::token::{Token, TokenKind};

use crate::roles::{ends_value, is_name, Role};

/// What to write between the tokens at `previous` and `next`, which sit
/// side by side on one line. `inside` is the bracket they are in, when it
/// was opened earlier on the same line.
///
/// Words and operators get one space between them. There is none just
/// inside `()` and `[]`, in an empty `{}`, before `,`, `:`, `.` and `?`,
/// after `.`, after a `-`, `!` or `~` that comes before a value, around type
/// arguments, or before the `(` of a call or parameter list. Fields written side by side without commas,
/// `{ first: A  second: B }`, are kept apart with two spaces.
pub(crate) fn space_between(
    tokens: &[Token],
    roles: &[Role],
    previous: usize,
    next: usize,
    inside: Option<&TokenKind>,
) -> &'static str {
    let before = &tokens[previous].kind;
    let after = &tokens[next].kind;

    if roles[previous] == Role::Unary {
        return "";
    }

    if matches!(
        before,
        TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot
    ) {
        return "";
    }

    if matches!(
        after,
        TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::Question
            | TokenKind::RightParen
            | TokenKind::RightBracket
    ) {
        return "";
    }

    if roles[next] == Role::TypeBracket {
        return "";
    }

    if roles[previous] == Role::TypeBracket && *before == TokenKind::Less {
        return "";
    }

    let opens_arguments = is_name(tokens, previous)
        || roles[previous] == Role::TypeBracket
        || *before == TokenKind::RightParen;

    if *after == TokenKind::LeftParen && opens_arguments {
        return "";
    }

    if *before == TokenKind::LeftBrace && *after == TokenKind::RightBrace {
        return "";
    }

    if starts_next_field(tokens, roles, previous, next, inside) {
        return "  ";
    }

    " "
}

/// Whether `next` starts a field, `second: B`, straight after another
/// field's value in braces or parentheses, with no comma between.
fn starts_next_field(
    tokens: &[Token],
    roles: &[Role],
    previous: usize,
    next: usize,
    inside: Option<&TokenKind>,
) -> bool {
    let in_fields = matches!(inside, Some(TokenKind::LeftBrace | TokenKind::LeftParen));

    let is_field_name = matches!(
        tokens[next].kind,
        TokenKind::Identifier(_) | TokenKind::Keyword(_)
    ) && tokens
        .get(next + 1)
        .is_some_and(|token| token.kind == TokenKind::Colon);

    in_fields && is_field_name && ends_value(tokens, roles, previous)
}
