//! Explains where a declaration belongs when it turns up in the wrong place.
//!
//! A file holds contexts and domains; a context holds most declarations; an
//! aggregate holds fields, entities, invariants, errors, events, commands and
//! factories.

use super::Parser;
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

/// Declarations written directly in a context.
const CONTEXT_ONLY: [Keyword; 15] = [
    Keyword::Publish,
    Keyword::Use,
    Keyword::Value,
    Keyword::Enum,
    Keyword::Const,
    Keyword::Identity,
    Keyword::Constraint,
    Keyword::Aggregate,
    Keyword::Fn,
    Keyword::Service,
    Keyword::Translation,
    Keyword::Policy,
    Keyword::Port,
    Keyword::Adapter,
    Keyword::Repository,
];

impl Parser {
    /// A declaration at the top of a file that belongs inside a context or an aggregate.
    pub(super) fn misplaced_in_file(&self) -> Option<Diagnostic> {
        let keyword = self.peeked_keyword()?;

        let message = if CONTEXT_ONLY.contains(&keyword) || keyword == Keyword::Error {
            format!(
                "`{}` is declared inside a context: `context Name {{ ... }}`",
                keyword.as_str()
            )
        } else {
            aggregate_only(keyword)?.to_string()
        };

        Some(Diagnostic::new(message, self.span()))
    }

    /// A declaration in a context that belongs inside an aggregate.
    pub(super) fn misplaced_in_context(&self) -> Option<Diagnostic> {
        let message = aggregate_only(self.peeked_keyword()?)?;
        Some(Diagnostic::new(message, self.span()))
    }

    /// A declaration in an aggregate that belongs in its context. A keyword
    /// followed by `:` is a mistaken field name, which `parse_field` reports.
    pub(super) fn misplaced_in_aggregate(&self) -> Option<Diagnostic> {
        let keyword = self.peeked_keyword()?;

        if !CONTEXT_ONLY.contains(&keyword) || self.peek_nth(1).kind == TokenKind::Colon {
            return None;
        }

        let message = match keyword {
            // Explained by `identity_in_fields`, which entities and values share.
            Keyword::Identity => return None,
            Keyword::Aggregate => "aggregates do not nest; declare each aggregate in the \
                                        context and refer to another by its identity"
                .to_string(),
            _ => format!(
                "`{}` is declared in the context, not inside an aggregate; an aggregate holds \
                 fields, entities, invariants, errors, events and commands",
                keyword.as_str()
            ),
        };

        Some(Diagnostic::new(message, self.span()))
    }

    /// `identity` where a field is expected. A capitalised name, `identity
    /// RoomId: Int`, is an identity type written in the wrong place; otherwise
    /// it is the old way of marking the identity field, `identity id: RoomId`.
    /// `owner` names the aggregate the field is in, when it is known.
    pub(super) fn identity_in_fields(&self, owner: Option<&str>) -> Option<Diagnostic> {
        if !self.check_keyword(Keyword::Identity) {
            return None;
        }

        let owner = owner.unwrap_or("Name");

        let message = match &self.peek_nth(1).kind {
            TokenKind::Identifier(name) if name.starts_with(char::is_uppercase) => format!(
                "an identity type is declared in the context, beside the aggregate: \
                 `identity {name} for {owner}: Int`; the field that holds it is written \
                 `id: {name}`"
            ),
            _ => format!(
                "an identity field needs no marker: write `id: {owner}Id`, and declare the \
                 type in the context with what it identifies: `identity {owner}Id for {owner}: Int`"
            ),
        };

        Some(Diagnostic::new(message, self.span()))
    }

    /// The next token, when it is a keyword.
    fn peeked_keyword(&self) -> Option<Keyword> {
        match self.peek().kind {
            TokenKind::Keyword(keyword) => Some(keyword),
            _ => None,
        }
    }
}

/// What to say about a declaration that only an aggregate holds.
fn aggregate_only(keyword: Keyword) -> Option<&'static str> {
    Some(match keyword {
        Keyword::Entity => {
            "an entity belongs inside an aggregate; a thing with identity on its own is an \
             `aggregate` with no entities"
        }
        Keyword::Event => "an event belongs inside the aggregate that emits it",
        Keyword::Command => "a command belongs inside the aggregate it changes",
        Keyword::Factory => "a factory belongs inside the aggregate it makes",
        Keyword::Invariant => "an invariant belongs inside the aggregate it protects",
        _ => return None,
    })
}
