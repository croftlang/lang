//! Functions, services, translations, commands, policies and constraints.

use super::{ParseResult, Parser};
use crate::ast::{ConstraintDecl, Expr, FunctionDecl, FunctionKind, Path, PolicyDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    /// Anything with a signature: a name, parameters, a return type with
    /// rules, a contract in braces and a body after `=`. Only the name is required.
    pub(super) fn parse_function(&mut self) -> ParseResult<FunctionDecl> {
        let doc = self.doc();
        let kind = match self.advance().kind {
            TokenKind::Keyword(Keyword::Fn) => FunctionKind::Function,
            TokenKind::Keyword(Keyword::Service) => FunctionKind::Service,
            TokenKind::Keyword(Keyword::Translation) => FunctionKind::Translation,
            TokenKind::Keyword(Keyword::Command) => FunctionKind::Command,
            _ => unreachable!("only called on a function keyword"),
        };
        let name = self.expect_identifier()?;
        let parameters = if self.check(&TokenKind::LeftParen) {
            Some(self.parse_parameters()?)
        } else {
            None
        };
        let (mut return_type, mut return_rules) = (None, Vec::new());
        if self.eat(&TokenKind::Colon) {
            return_type = Some(self.parse_type()?);
            return_rules = self.parse_rules()?;
        }
        let (preconditions, uses) = self.parse_contract()?;
        let body = if self.eat(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(FunctionDecl {
            doc,
            kind,
            name,
            parameters,
            return_type,
            return_rules,
            preconditions,
            uses,
            body,
        })
    }

    /// The braces between a signature and its body: preconditions, one per
    /// line, and the ports a service `uses`. Left out when empty.
    fn parse_contract(&mut self) -> ParseResult<(Vec<Expr>, Vec<Path>)> {
        let (mut preconditions, mut uses) = (Vec::new(), Vec::new());
        if !self.eat(&TokenKind::LeftBrace) {
            return Ok((preconditions, uses));
        }
        while !self.eat(&TokenKind::RightBrace) {
            if self.eat_keyword(Keyword::Uses) {
                loop {
                    uses.push(self.parse_path()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
            } else {
                preconditions.push(self.parse_expression()?);
            }
        }
        Ok((preconditions, uses))
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
