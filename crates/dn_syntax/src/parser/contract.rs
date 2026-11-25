//! A contract: the braces between a signature and its body.

use super::{ParseResult, Parser};
use crate::ast::{Expr, Parameter, Path};
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

impl Parser {
    /// The braces between a signature and its body: preconditions, one per
    /// line, constraints for parameters, and the ports a service `calls`. Left out
    /// when empty.
    pub(super) fn parse_contract(
        &mut self,
        mut parameters: Option<&mut Vec<Parameter>>,
    ) -> ParseResult<(Vec<Expr>, Vec<Path>)> {
        let (mut preconditions, mut calls) = (Vec::new(), Vec::new());
        if !self.eat(&TokenKind::LeftBrace) {
            return Ok((preconditions, calls));
        }
        while !self.eat(&TokenKind::RightBrace) {
            self.reject_uses()?;
            if self.eat_keyword(Keyword::Calls) {
                loop {
                    calls.push(self.parse_path()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
            } else if self.peek_nth(1).kind == TokenKind::Keyword(Keyword::Where) {
                self.parse_parameter_constraints(parameters.as_deref_mut())?;
            } else {
                preconditions.push(self.parse_expression()?);
            }
        }
        Ok((preconditions, calls))
    }

    /// The ports a body calls were once listed with `uses`, which reads too
    /// much like `use`, so the old word gets an error that names the new one.
    pub(super) fn reject_uses(&mut self) -> ParseResult<()> {
        if self.check_word("uses") {
            let message =
                "the ports a body calls are listed with `calls`: `calls Screen, Keyboard`";
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
