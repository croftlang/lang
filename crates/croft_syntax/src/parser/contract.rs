//! A contract: the braces between a signature and its body.

use super::{ParseResult, Parser};
use crate::ast::{CalledPort, Expr, Parameter};
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

impl Parser {
    /// The braces between a signature and its body: contract lines such as
    /// `from != to`, constraints on parameters, and the ports the body `calls`,
    /// one per line. Left out when empty.
    pub(super) fn parse_contract(
        &mut self,
        mut parameters: Option<&mut Vec<Parameter>>,
    ) -> ParseResult<(Vec<Expr>, Vec<CalledPort>)> {
        let (mut preconditions, mut calls) = (Vec::new(), Vec::new());

        if !self.eat(&TokenKind::LeftBrace) {
            return Ok((preconditions, calls));
        }

        while !self.eat(&TokenKind::RightBrace) {
            self.reject_uses()?;

            if self.check_keyword(Keyword::Calls) {
                calls.extend(self.parse_calls()?);
            } else if self.peek_nth(1).kind == TokenKind::Keyword(Keyword::Where) {
                self.parse_parameter_constraints(parameters.as_deref_mut())?;
            } else {
                preconditions.push(self.parse_expression()?);
            }
        }

        Ok((preconditions, calls))
    }

    /// `calls { Keyboard as keyboard } from Dungeon`: the ports a body may
    /// call, named as `use` names them. `from` is left out for a port the code
    /// can already see, and must be on the same line, since the next line of a
    /// contract may start with a parameter called `from`.
    pub(super) fn parse_calls(&mut self) -> ParseResult<Vec<CalledPort>> {
        self.expect_keyword(Keyword::Calls)?;

        if !self.check(&TokenKind::LeftBrace) {
            let example = match &self.peek().kind {
                TokenKind::Identifier(name) => name.clone(),
                _ => "Screen, Keyboard".to_string(),
            };

            let message = format!("the ports a body calls go in braces: `calls {{ {example} }}`");
            return Err(Diagnostic::new(message, self.span()));
        }

        let names = self.parse_imported_names()?;
        let mut from = None;

        if self.check_word("from") && !self.peek().newline_before {
            self.advance();
            from = Some(self.parse_path()?);
        }

        let calls = names
            .into_iter()
            .map(|name| CalledPort {
                name,
                from: from.clone(),
            })
            .collect();

        Ok(calls)
    }

    /// The ports a body calls were once listed with `uses`, which reads too
    /// much like `use`, so the old word gets an error that names the new one.
    pub(super) fn reject_uses(&mut self) -> ParseResult<()> {
        if self.check_word("uses") {
            let message =
                "the ports a body calls are listed with `calls`: `calls { Screen, Keyboard }`";
            return Err(Diagnostic::new(message, self.span()));
        }

        Ok(())
    }

    /// `damage where Range(1, 20)` in a contract: the same constraints as writing
    /// them after the parameter's type, so they are added to the parameter.
    fn parse_parameter_constraints(
        &mut self,
        parameters: Option<&mut Vec<Parameter>>,
    ) -> ParseResult<()> {
        let name = self.expect_identifier()?;
        let constraints = self.parse_constraints()?;
        let parameter =
            parameters.and_then(|all| all.iter_mut().find(|p| p.name.name == name.name));

        match parameter {
            Some(parameter) => parameter.constraints.extend(constraints),
            None => {
                let message = format!(
                    "`{}` is not a parameter, so it can't have constraints here",
                    name.name
                );

                return Err(Diagnostic::new(message, name.span));
            }
        }

        Ok(())
    }
}
