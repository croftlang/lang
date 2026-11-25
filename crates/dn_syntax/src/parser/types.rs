//! Paths, types, fields, parameters and `where` rules.

use super::{ParseResult, Parser};
use crate::ast::{Field, Parameter, Path, TypeExpr};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_path(&mut self) -> ParseResult<Path> {
        let mut segments = vec![self.expect_identifier()?];
        while self.eat(&TokenKind::Dot) {
            segments.push(self.expect_identifier()?);
        }
        Ok(Path { segments })
    }

    /// `{ RoomId, Room.EnemyDefeated }`, with optional commas.
    pub(super) fn parse_path_list(&mut self) -> ParseResult<Vec<Path>> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut paths = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            paths.push(self.parse_path()?);
            self.eat(&TokenKind::Comma);
        }
        Ok(paths)
    }

    pub(super) fn parse_type(&mut self) -> ParseResult<TypeExpr> {
        let path = self.parse_path()?;
        let mut arguments = Vec::new();
        if self.eat(&TokenKind::Less) {
            loop {
                arguments.push(self.parse_type()?);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.close_type_arguments()?;
        }
        Ok(TypeExpr { path, arguments })
    }

    /// A field: `identity id: RoomId` or `hit_points: Int where Range(0, 100)`.
    pub(super) fn parse_field(&mut self) -> ParseResult<Field> {
        let doc = self.doc();
        let is_identity = self.eat_keyword(Keyword::Identity);
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let rules = self.parse_rules()?;
        let value = if self.eat(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Field {
            value,
            doc,
            name,
            ty,
            rules,
            is_identity,
        })
    }

    /// Fields in braces, separated by commas or just spaces.
    pub(super) fn parse_field_block(&mut self) -> ParseResult<Vec<Field>> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut fields = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            fields.push(self.parse_field()?);
            self.eat(&TokenKind::Comma);
        }
        Ok(fields)
    }

    /// An enum variant's fields: `Coins(count: Int where Range(1, 99))`.
    pub(super) fn parse_variant_fields(&mut self) -> ParseResult<Vec<Field>> {
        self.expect(&TokenKind::LeftParen)?;
        let mut fields = Vec::new();
        while !self.eat(&TokenKind::RightParen) {
            fields.push(self.parse_field()?);
            self.eat(&TokenKind::Comma);
        }
        Ok(fields)
    }

    /// `(enemy_id: EnemyId, damage: Int where Range(1, 20))`
    pub(super) fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        self.expect(&TokenKind::LeftParen)?;
        let mut parameters = Vec::new();
        while !self.eat(&TokenKind::RightParen) {
            let name = self.expect_identifier()?;
            let mut types = Vec::new();
            if self.eat(&TokenKind::Colon) {
                types.push(self.parse_type()?);
                while self.eat_keyword(Keyword::Or) {
                    types.push(self.parse_type()?);
                }
            }
            let rules = self.parse_rules()?;
            parameters.push(Parameter { name, types, rules });
            if !self.eat(&TokenKind::Comma) {
                self.expect(&TokenKind::RightParen)?;
                break;
            }
        }
        Ok(parameters)
    }

    /// The `>` that ends type arguments. In `List<List<Int>>` the lexer reads
    /// `>>` as one token, so it is split, leaving the second `>` for the outer type.
    fn close_type_arguments(&mut self) -> ParseResult<()> {
        if self.check(&TokenKind::ShiftRight) {
            self.split_shift_right();
            return Ok(());
        }
        self.expect(&TokenKind::Greater)?;
        Ok(())
    }
}
