//! Formatting Croft source files, in one layout with no settings.
//!
//! The formatter keeps every token, comment and line break where it was
//! written, and only changes the space around them: each line is indented
//! by the brackets it is inside, tokens on a line are spaced the same way
//! everywhere, and blank lines are cut to one. It reads tokens rather than
//! the syntax tree, since the tree leaves out brackets, comments and how
//! numbers were written.
//!
//! A file that does not parse is not formatted. Before giving the result
//! back, the formatter reads it again and checks that it has the same
//! tokens, line breaks and comments as the file it was given.

mod layout;
mod roles;
mod spacing;
#[cfg(test)]
mod tests;

use croft_syntax::lexer::{tokenize_with_comments, TokenizedSource};
use croft_syntax::token::TokenKind;
use croft_syntax::{parse, Diagnostic, SourceId, Span};

/// Formats one source file. Fails with the parser's error when the file
/// does not parse.
pub fn format_source(source: &str) -> Result<String, Diagnostic> {
    let source_id = SourceId(0);

    parse(source, source_id)?;

    let tokenized = tokenize_with_comments(source, source_id)?;
    let formatted = layout::lay_out(source, &tokenized);

    check_same_program(&tokenized, &formatted)?;

    Ok(formatted)
}

/// Checks that the formatted text reads as the same program: the same
/// tokens, the same line breaks between them, the same doc comments and
/// the same comments.
fn check_same_program(before: &TokenizedSource, formatted: &str) -> Result<(), Diagnostic> {
    let end = before
        .tokens
        .last()
        .map_or(Span::default(), |token| token.span);

    let after = tokenize_with_comments(formatted, SourceId(0)).map_err(|_| changed(end))?;

    parse(formatted, SourceId(0)).map_err(|_| changed(end))?;

    if before.tokens.len() != after.tokens.len() {
        return Err(changed(end));
    }

    for (old, new) in before.tokens.iter().zip(&after.tokens) {
        let is_word = matches!(old.kind, TokenKind::Identifier(_) | TokenKind::Keyword(_));
        // The file always ends with a line break, which the parser ignores.
        let is_end = old.kind == TokenKind::EndOfFile;

        if old.kind != new.kind
            || (!is_end && old.newline_before != new.newline_before)
            || (is_word && old.doc != new.doc)
        {
            return Err(changed(old.span));
        }
    }

    let old_comments = before.comments.iter().map(|comment| &comment.text);
    let new_comments = after.comments.iter().map(|comment| &comment.text);

    if !old_comments.eq(new_comments) {
        return Err(changed(end));
    }

    Ok(())
}

fn changed(span: Span) -> Diagnostic {
    Diagnostic::new(
        "formatting would change what this file means, so it was left alone; \
         this is a bug in the formatter, please report it",
        span,
    )
}
