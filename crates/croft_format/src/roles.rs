//! What the tokens that mean more than one thing are doing: whether `<` and
//! `>` hold type arguments or compare, and whether `-` negates or subtracts.

use croft_syntax::keyword::Keyword;
use croft_syntax::token::{Token, TokenKind};

/// The part a token plays, where its kind alone does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Plain,
    /// `<`, `>` or `>>` around type arguments, as in `List<List<Int>>`.
    TypeBracket,
    /// `-`, `!` or `~` before a value.
    Unary,
}

/// The declarations whose name may be followed by type parameters.
const GENERIC_DECLARATIONS: [Keyword; 7] = [
    Keyword::Fn,
    Keyword::Service,
    Keyword::Translation,
    Keyword::Command,
    Keyword::Factory,
    Keyword::Value,
    Keyword::Entity,
];

/// The role of every token, one for each.
pub(crate) fn find_roles(tokens: &[Token]) -> Vec<Role> {
    let mut roles = vec![Role::Plain; tokens.len()];

    for index in 0..tokens.len() {
        if tokens[index].kind == TokenKind::Less && starts_type_arguments(tokens, index) {
            mark_type_arguments(tokens, index, &mut roles);
        }
    }

    for index in 0..tokens.len() {
        let is_unary = match tokens[index].kind {
            TokenKind::Not | TokenKind::Tilde => true,
            TokenKind::Minus => {
                index == 0 || tokens[index].newline_before || !ends_value(tokens, &roles, index - 1)
            }
            _ => false,
        };

        if is_unary {
            roles[index] = Role::Unary;
        }
    }

    roles
}

/// Whether the token is a name: an identifier, or a keyword used as a field
/// name after `.`, as in `score.value`.
pub(crate) fn is_name(tokens: &[Token], index: usize) -> bool {
    match tokens[index].kind {
        TokenKind::Identifier(_) => true,
        TokenKind::Keyword(_) => index > 0 && tokens[index - 1].kind == TokenKind::Dot,
        _ => false,
    }
}

/// Whether a value can end with this token, so a `-` after it subtracts.
pub(crate) fn ends_value(tokens: &[Token], roles: &[Role], index: usize) -> bool {
    match tokens[index].kind {
        TokenKind::Identifier(_)
        | TokenKind::Int(_)
        | TokenKind::Float(_)
        | TokenKind::String(_)
        | TokenKind::RightParen
        | TokenKind::RightBracket
        | TokenKind::RightBrace
        | TokenKind::Question => true,
        TokenKind::Greater | TokenKind::ShiftRight => roles[index] == Role::TypeBracket,
        TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::Break) => true,
        TokenKind::Keyword(_) => is_name(tokens, index),
        _ => false,
    }
}

/// Whether the `<` at `index` opens type arguments. It must follow a type
/// name, `List<`, or the name of a generic declaration, `fn first<`, and
/// hold only what types are written with before its `>`.
fn starts_type_arguments(tokens: &[Token], index: usize) -> bool {
    if index == 0 || tokens[index].newline_before {
        return false;
    }

    let TokenKind::Identifier(name) = &tokens[index - 1].kind else {
        return false;
    };

    let names_declaration = index >= 2
        && matches!(
            tokens[index - 2].kind,
            TokenKind::Keyword(keyword) if GENERIC_DECLARATIONS.contains(&keyword)
        );

    if !is_type_name(name) && !names_declaration {
        return false;
    }

    closing_bracket(tokens, index).is_some()
}

/// The index of the `>` or `>>` that closes the `<` at `open`, when only
/// names, dots, commas, numbers and nested type arguments come between.
fn closing_bracket(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth: i32 = 1;

    for (index, token) in tokens.iter().enumerate().skip(open + 1) {
        if token.newline_before {
            return None;
        }

        match token.kind {
            TokenKind::Identifier(_) | TokenKind::Dot | TokenKind::Comma | TokenKind::Int(_) => {}
            TokenKind::Less => depth += 1,
            TokenKind::Greater => depth -= 1,
            TokenKind::ShiftRight => depth -= 2,
            _ => return None,
        }

        if depth <= 0 {
            return Some(index);
        }
    }

    None
}

fn mark_type_arguments(tokens: &[Token], open: usize, roles: &mut [Role]) {
    let Some(close) = closing_bracket(tokens, open) else {
        return;
    };

    for index in open..=close {
        if matches!(
            tokens[index].kind,
            TokenKind::Less | TokenKind::Greater | TokenKind::ShiftRight
        ) {
            roles[index] = Role::TypeBracket;
        }
    }
}

/// A type is named in PascalCase, `Room`. A constant, `MAX_HP`, has no small letters.
fn is_type_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase()) && name.chars().any(|c| c.is_lowercase())
}
