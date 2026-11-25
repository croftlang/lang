//! Functions, services, translations, commands, factories, policies and constraints.

use super::{ParseResult, Parser};
use crate::ast::{ConstraintDecl, FunctionDecl, FunctionKind, PolicyDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// Anything with a signature: a name, parameters, a return type with
    /// constraints, a contract in braces and a body after `=`. Only the name is required.
    pub(super) fn parse_function(&mut self) -> ParseResult<FunctionDecl> {
        let doc = self.doc();

        let kind = match self.advance().kind {
            TokenKind::Keyword(Keyword::Fn) => FunctionKind::Function,
            TokenKind::Keyword(Keyword::Service) => FunctionKind::Service,
            TokenKind::Keyword(Keyword::Translation) => FunctionKind::Translation,
            TokenKind::Keyword(Keyword::Command) => FunctionKind::Command,
            TokenKind::Keyword(Keyword::Factory) => FunctionKind::Factory,
            _ => unreachable!("only called on a function keyword"),
        };

        let name = self.expect_identifier()?;
        let type_parameters = self.parse_type_parameters()?;

        let mut parameters = if self.check(&TokenKind::LeftParen) {
            Some(self.parse_parameters()?)
        } else {
            None
        };

        let (mut return_type, mut return_constraints) = (None, Vec::new());

        if self.eat(&TokenKind::Colon) {
            return_type = Some(self.parse_type()?);
            return_constraints = self.parse_constraints()?;
        }

        let (preconditions, calls) = self.parse_contract(parameters.as_mut())?;

        let body = if self.eat(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(FunctionDecl {
            doc,
            kind,
            name,
            type_parameters,
            parameters,
            return_type,
            return_constraints,
            preconditions,
            calls,
            body,
        })
    }

    /// `policy DropLootOnDefeat when Room.EnemyDefeated { ... }`
    pub(super) fn parse_policy(&mut self) -> ParseResult<PolicyDecl> {
        let doc = self.doc();

        self.expect_keyword(Keyword::Policy)?;

        let name = self.expect_identifier()?;

        self.expect_word("when")?;

        let event = self.parse_path()?;
        let body = self.parse_block()?;

        Ok(PolicyDecl {
            doc,
            name,
            event,
            body,
        })
    }

    /// `constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0`
    pub(super) fn parse_constraint(&mut self) -> ParseResult<ConstraintDecl> {
        let doc = self.doc();

        self.expect_keyword(Keyword::Constraint)?;

        let name = self.expect_identifier()?;
        let parameters = self.parse_parameters()?;

        self.expect(&TokenKind::Equal)?;

        let body = self.parse_expression()?;

        Ok(ConstraintDecl {
            doc,
            name,
            parameters,
            body,
        })
    }
}
