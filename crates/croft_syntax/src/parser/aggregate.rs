//! Aggregates and what they declare: fields, entities, invariants, errors,
//! events, commands and factories.

use super::{ParseResult, Parser};
use crate::ast::{AggregateDecl, Invariant, RecordDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// An aggregate in braces or, in a file with a context header, as a header
    /// line that claims the rest of the file.
    pub(super) fn parse_aggregate(&mut self, context_braced: bool) -> ParseResult<AggregateDecl> {
        let doc = self.doc();

        self.expect_keyword(Keyword::Aggregate)?;

        let name = self.expect_identifier()?;
        let braced = context_braced || self.check(&TokenKind::LeftBrace);

        let end = if braced {
            TokenKind::RightBrace
        } else {
            TokenKind::EndOfFile
        };

        if braced {
            self.expect(&TokenKind::LeftBrace)?;
        }

        let mut aggregate = AggregateDecl {
            doc,
            name,
            fields: Vec::new(),
            invariants: Vec::new(),
            entities: Vec::new(),
            errors: Vec::new(),
            events: Vec::new(),
            commands: Vec::new(),
            factories: Vec::new(),
        };

        while !self.check(&end) {
            self.parse_aggregate_member(&mut aggregate)?;
        }

        self.advance();

        Ok(aggregate)
    }

    /// One member of an aggregate, added to the list it belongs in.
    fn parse_aggregate_member(&mut self, aggregate: &mut AggregateDecl) -> ParseResult<()> {
        let misplaced = self
            .misplaced_in_aggregate()
            .or_else(|| self.identity_in_fields(Some(&aggregate.name.name)));

        if let Some(misplaced) = misplaced {
            return Err(misplaced);
        }

        match self.peek().kind {
            TokenKind::Keyword(Keyword::Entity) => {
                aggregate.entities.push(self.parse_value(Keyword::Entity)?)
            }
            TokenKind::Keyword(Keyword::Invariant) => {
                aggregate.invariants.push(self.parse_invariant()?)
            }
            TokenKind::Keyword(Keyword::Error) if self.peek_nth(1).kind != TokenKind::Colon => {
                aggregate.errors.push(self.parse_record(Keyword::Error)?)
            }
            TokenKind::Keyword(Keyword::Event) if self.peek_nth(1).kind != TokenKind::Colon => {
                aggregate.events.push(self.parse_record(Keyword::Event)?)
            }
            TokenKind::Keyword(Keyword::Command) => aggregate.commands.push(self.parse_function()?),
            TokenKind::Keyword(Keyword::Factory) => {
                aggregate.factories.push(self.parse_function()?)
            }
            TokenKind::EndOfFile => return Err(self.unexpected("`}`")),
            _ => aggregate.fields.push(self.parse_field()?),
        }

        Ok(())
    }

    /// `invariant ExitClear or ExitBlocked`
    pub(super) fn parse_invariant(&mut self) -> ParseResult<Invariant> {
        self.expect_keyword(Keyword::Invariant)?;

        let constraint = self.parse_constraint_use()?;

        let error = if self.eat_keyword(Keyword::Or) {
            Some(self.parse_or_operand()?)
        } else {
            None
        };

        Ok(Invariant { constraint, error })
    }

    /// An error or event: `event EnemyDamaged { enemy_id: EnemyId  remaining: Int }`.
    pub(super) fn parse_record(&mut self, keyword: Keyword) -> ParseResult<RecordDecl> {
        let doc = self.doc();

        self.expect_keyword(keyword)?;

        let name = self.expect_identifier()?;

        let fields = if self.check(&TokenKind::LeftBrace) && !self.peek().newline_before {
            self.parse_field_block()?
        } else {
            Vec::new()
        };

        Ok(RecordDecl { doc, name, fields })
    }
}
