//! Files and contexts.
//!
//! A context is written inline, `context Dungeon { ... }`, or as a header line,
//! `context Dungeon`, which claims the rest of the file.

use super::{ParseResult, Parser};
use crate::ast::{ContextDecl, ContextItem, File, ImportDecl};
use crate::keyword::Keyword;
use crate::span::Diagnostic;
use crate::token::TokenKind;

impl Parser {
    /// Every context and domain in the file.
    pub(super) fn parse_file(&mut self) -> ParseResult<File> {
        let mut file = File::default();

        while !self.check(&TokenKind::EndOfFile) {
            if self.check_keyword(Keyword::Domain) {
                file.domains.push(self.parse_domain()?);
                continue;
            }

            if !self.check_keyword(Keyword::Context) {
                if let Some(misplaced) = self.misplaced_in_file() {
                    return Err(misplaced);
                }

                return Err(self.unexpected("`context` or `domain`"));
            }

            file.contexts.push(self.parse_context()?);
        }

        Ok(file)
    }

    /// A context, in braces or as a header line.
    fn parse_context(&mut self) -> ParseResult<ContextDecl> {
        let doc = self.doc();
        self.expect_keyword(Keyword::Context)?;

        let name = self.expect_identifier()?;
        let braced = self.eat(&TokenKind::LeftBrace);

        let end = if braced {
            TokenKind::RightBrace
        } else {
            TokenKind::EndOfFile
        };

        let mut items = Vec::new();

        while !self.check(&end) {
            if self.check(&TokenKind::EndOfFile) {
                return Err(self.unexpected("`}`"));
            }

            items.push(self.parse_context_item(braced)?);
        }

        if braced {
            self.advance();
        }

        Ok(ContextDecl { doc, name, items })
    }

    /// One declaration in a context. `braced` says whether the context is in
    /// braces, since an aggregate inside a braced context must be braced too.
    fn parse_context_item(&mut self, braced: bool) -> ParseResult<ContextItem> {
        self.reject_old_term_list()?;

        let TokenKind::Keyword(keyword) = self.peek().kind else {
            return Err(self.unexpected("a declaration"));
        };

        let item = match keyword {
            Keyword::Export => {
                self.advance();
                ContextItem::Export(self.parse_path_list()?)
            }
            Keyword::Import => ContextItem::Import(self.parse_import()?),
            Keyword::Value => ContextItem::Value(self.parse_value(Keyword::Value)?),
            Keyword::Enum => ContextItem::Enum(self.parse_enum()?),
            Keyword::Error => ContextItem::Error(self.parse_record(Keyword::Error)?),
            Keyword::Const => ContextItem::Const(self.parse_const()?),
            Keyword::Identity => ContextItem::Identity(self.parse_identity()?),
            Keyword::Constraint => ContextItem::Constraint(self.parse_constraint()?),
            Keyword::Aggregate => ContextItem::Aggregate(self.parse_aggregate(braced)?),
            Keyword::Fn | Keyword::Service | Keyword::Translation => {
                ContextItem::Function(self.parse_function()?)
            }
            Keyword::Policy => ContextItem::Policy(self.parse_policy()?),
            Keyword::Port => ContextItem::Port(self.parse_port()?),
            Keyword::Adapter => ContextItem::Adapter(self.parse_adapter()?),
            Keyword::Repository => ContextItem::Repository(self.parse_repository()?),
            _ => {
                return Err(self
                    .misplaced_in_context()
                    .unwrap_or_else(|| self.unexpected("a declaration")));
            }
        };

        Ok(item)
    }

    /// `import { EnemyId, Room.EnemyDefeated } from Dungeon`
    pub(super) fn parse_import(&mut self) -> ParseResult<ImportDecl> {
        self.expect_keyword(Keyword::Import)?;

        let names = self.parse_imported_names()?;

        self.expect_word("from")?;

        let from = self.parse_path()?;
        Ok(ImportDecl { names, from })
    }

    /// Term lists were once written `publish { ... }` and `use { ... } from`,
    /// so the old words get an error that rewrites the list with the new one.
    pub(super) fn reject_old_term_list(&self) -> ParseResult<()> {
        let new = if self.check_word("publish") {
            "export"
        } else if self.check_word("use") {
            "import"
        } else {
            return Ok(());
        };

        if self.peek_nth(1).kind != TokenKind::LeftBrace {
            return Ok(());
        }

        let (names, mut offset) = self.written_names(2);
        let mut rewritten = format!("{new} {{ {names} }}");

        if offset > 2 && self.peek_nth(offset).kind == TokenKind::RightBrace {
            offset += 1;

            if new == "import" && is_word(&self.peek_nth(offset).kind, "from") {
                let (from, _) = self.written_names(offset + 1);
                rewritten = format!("{rewritten} from {from}");
            }
        }

        let old = if new == "export" { "publish" } else { "use" };
        let message = format!("`{old}` is now `{new}`: `{rewritten}`");

        Err(Diagnostic::new(message, self.span()))
    }

    /// The names, dots, commas and `as` written from token `offset` on, as
    /// text, and the offset of the first token after them.
    fn written_names(&self, mut offset: usize) -> (String, usize) {
        let mut text = String::new();
        let mut after_name = false;

        loop {
            match &self.peek_nth(offset).kind {
                TokenKind::Identifier(name) => {
                    if after_name {
                        text.push(' ');
                    }

                    text.push_str(name);
                    after_name = true;
                }
                TokenKind::Dot => {
                    text.push('.');
                    after_name = false;
                }
                TokenKind::Comma => {
                    text.push_str(", ");
                    after_name = false;
                }
                _ => return (text, offset),
            }

            offset += 1;
        }
    }
}

/// Whether `kind` is the identifier `word`.
fn is_word(kind: &TokenKind, word: &str) -> bool {
    matches!(kind, TokenKind::Identifier(name) if name == word)
}
