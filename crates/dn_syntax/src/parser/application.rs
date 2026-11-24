//! The application: which adapter fills each port on each target.

use super::{ParseResult, Parser};
use crate::ast::{ApplicationDecl, TargetDecl};
use crate::keyword::Keyword;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_application(&mut self) -> ParseResult<ApplicationDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Application)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        self.expect_word("start")?;
        let start = self.parse_path()?;
        let mut targets = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            targets.push(self.parse_target()?);
        }
        Ok(ApplicationDecl {
            doc,
            name,
            start,
            targets,
        })
    }

    /// `target spectrum_48k { Screen -> SpectrumScreen }`
    fn parse_target(&mut self) -> ParseResult<TargetDecl> {
        self.expect_word("target")?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;
        let mut bindings = Vec::new();
        while !self.eat(&TokenKind::RightBrace) {
            let port = self.parse_path()?;
            self.expect(&TokenKind::Arrow)?;
            bindings.push((port, self.parse_path()?));
        }
        Ok(TargetDecl { name, bindings })
    }
}
