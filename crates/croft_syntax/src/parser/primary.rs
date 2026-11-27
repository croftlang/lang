//! Literals, names, lambdas (`e -> e.id`, `(total, item) -> total + item`), brackets, and the `issue`, `emit` and `native` forms.

use super::{ParseResult, Parser};
use crate::ast::{Expr, ExprKind, Issue};
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

impl Parser {
    /// What a postfix expression starts with: a literal, a name, a lambda, an
    /// expression in brackets, a block, or a form that starts with a keyword.
    pub(super) fn parse_primary(&mut self) -> ParseResult<Expr> {
        let span = self.span();

        let kind = match self.peek().kind.clone() {
            TokenKind::Int(value) => self.literal(ExprKind::Int(value)),
            TokenKind::Float(value) => self.literal(ExprKind::Float(value)),
            TokenKind::String(value) => self.literal(ExprKind::String(value)),
            TokenKind::Keyword(Keyword::True) => self.literal(ExprKind::Bool(true)),
            TokenKind::Keyword(Keyword::False) => self.literal(ExprKind::Bool(false)),
            TokenKind::LeftParen if self.starts_lambda_in_brackets() => {
                self.advance();

                let first = self.expect_identifier()?;

                if self.check(&TokenKind::RightParen) {
                    let message = format!(
                        "a lambda with one parameter is written without brackets: `{} -> ...`",
                        first.name
                    );
                    return Err(Diagnostic::new(message, first.span));
                }

                let mut parameters = vec![first];

                while self.eat(&TokenKind::Comma) {
                    parameters.push(self.expect_identifier()?);
                }

                self.expect(&TokenKind::RightParen)?;
                self.expect(&TokenKind::Arrow)?;

                ExprKind::Lambda(parameters, Box::new(self.parse_expression()?))
            }
            TokenKind::LeftParen => {
                self.advance();

                let inner = self.with_records(true, Self::parse_expression)?;

                self.expect(&TokenKind::RightParen)?;
                return Ok(inner);
            }
            TokenKind::LeftBrace => return self.parse_block(),
            TokenKind::LeftBracket => self.parse_list_or_map()?,
            TokenKind::Keyword(Keyword::If) => return self.parse_if(),
            TokenKind::Keyword(Keyword::Match) => return self.parse_match(),
            TokenKind::Keyword(Keyword::Loop) => return self.parse_loop(),
            TokenKind::Keyword(Keyword::Break) => self.literal(ExprKind::Break),
            TokenKind::Keyword(Keyword::Issue) => ExprKind::Issue(self.parse_issue()?),
            TokenKind::Keyword(Keyword::Emit) => {
                self.advance();

                let event = self.parse_path()?;

                let fields = if self.check(&TokenKind::LeftBrace) && !self.peek().newline_before {
                    self.parse_field_values()?
                } else {
                    Vec::new()
                };

                ExprKind::Emit(event, fields)
            }
            TokenKind::Keyword(Keyword::Native) => {
                self.advance();
                let function = self.expect_identifier()?;
                ExprKind::Native(function, self.parse_arguments()?)
            }
            _ if self.is_identifier() && self.peek_nth(1).kind == TokenKind::Arrow => {
                let parameter = self.expect_identifier()?;
                self.advance();
                ExprKind::Lambda(vec![parameter], Box::new(self.parse_expression()?))
            }
            _ if self.is_identifier() => ExprKind::Name(self.expect_identifier()?.name),
            _ => return Err(self.unexpected("an expression")),
        };

        Ok(self.make_expr(kind, span))
    }

    /// Whether the next tokens are `(total, item) ->`: the parameters of a
    /// lambda in brackets, which a lambda with more than one parameter needs.
    /// A single name in brackets before `->` counts too, so that it can be
    /// rejected with the form it should take.
    fn starts_lambda_in_brackets(&self) -> bool {
        let mut offset = 1;

        loop {
            if !matches!(self.peek_nth(offset).kind, TokenKind::Identifier(_)) {
                return false;
            }

            match self.peek_nth(offset + 1).kind {
                TokenKind::Comma => offset += 2,
                TokenKind::RightParen => {
                    return self.peek_nth(offset + 2).kind == TokenKind::Arrow;
                }
                _ => return false,
            }
        }
    }

    /// Reads the literal's token and gives back `kind`.
    fn literal(&mut self, kind: ExprKind) -> ExprKind {
        self.advance();
        kind
    }

    /// `issue Attack(enemy_id: target, damage: 5) on room`
    fn parse_issue(&mut self) -> ParseResult<Issue> {
        self.expect_keyword(Keyword::Issue)?;

        let command = self.expect_identifier()?;

        let arguments = if self.check(&TokenKind::LeftParen) {
            self.parse_arguments()?
        } else {
            Vec::new()
        };

        self.expect_word("on")?;

        let target = Box::new(self.parse_postfix(false)?);

        Ok(Issue {
            command,
            arguments,
            target,
        })
    }
}
