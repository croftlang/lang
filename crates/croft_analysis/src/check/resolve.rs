//! Finding what a name refers to.
//!
//! A name is looked up in the current aggregate, then the context, then the
//! names the context imports with `use`. A name can also be written with its
//! context in front, `Dungeon.Room.EnemyDefeated`, if that context publishes it.

use crate::standard::{CONSOLE_CONTEXT, STORAGE_CONTEXT};
use croft_syntax::ast::Path;

use crate::facts::Reference;

use super::Checker;
use crate::program::Context;
use crate::symbols::SymbolKind;
pub(crate) use crate::types::Resolved;

/// Where a name is being looked up from.
#[derive(Clone, Copy)]
pub(crate) struct Scope<'a> {
    pub context: &'a Context,
    pub aggregate: Option<&'a str>,
}

impl<'a> Checker<'a> {
    /// The declaration `name` in `context`, if that context declares it.
    pub(super) fn lookup(&self, context: &str, name: &str) -> Option<Resolved> {
        let symbol = self.tables.get(context)?.get(name)?;
        let (context, name) = (context.to_string(), name.to_string());

        Some(Resolved {
            context,
            name,
            kind: symbol.kind,
        })
    }

    /// Whether `context` publishes `name`.
    pub(super) fn is_published(&self, context: &str, name: &str) -> bool {
        let Some(context) = self.program.contexts.get(context) else {
            return false;
        };

        context.publish.iter().any(|path| path.text() == name)
    }

    /// Finds what a path names, and notes it for editors. In `Room.Error`,
    /// `Room` is noted too, as the aggregate.
    pub(super) fn resolve(&self, scope: &Scope, path: &Path) -> Option<Resolved> {
        let found = self.find_path(scope, path)?;
        let mut references = self.references.borrow_mut();

        for end in 1..path.segments.len() {
            let prefix = Path {
                segments: path.segments[..end].to_vec(),
            };

            if let Some(target) = self.find_path(scope, &prefix) {
                let segment = &path.segments[end - 1];

                references.push(Reference {
                    span: segment.span,
                    name: segment.name.clone(),
                    target,
                });
            }
        }

        let last = path.last();

        references.push(Reference {
            span: last.span,
            name: last.name.clone(),
            target: found.clone(),
        });

        Some(found)
    }

    /// Finds what a path names, without noting it for editors: in the
    /// aggregate, the context, its imports, the standard contexts that need no
    /// import, then a context written in front.
    pub(super) fn find_path(&self, scope: &Scope, path: &Path) -> Option<Resolved> {
        let text = path.text();
        let local = scope.context.name.name.as_str();

        if let Some(aggregate) = scope.aggregate {
            if let Some(found) = self.lookup(local, &format!("{aggregate}.{text}")) {
                return Some(found);
            }
        }

        if let Some(found) = self.lookup(local, &text) {
            return Some(found);
        }

        for use_decl in &scope.context.uses {
            if let Some(name) = use_decl.names.iter().find(|name| name.local_name() == text) {
                return self.lookup(&use_decl.from.text(), &name.path.text());
            }
        }

        // `StorageError` and its errors, and `Console`, `Arguments`,
        // `Environment` and `Process`, need no import.
        for standard in [STORAGE_CONTEXT, CONSOLE_CONTEXT] {
            if self.is_published(standard, &text) {
                return self.lookup(standard, &text);
            }
        }

        let (first, rest) = path.segments.split_first()?;
        let rest: Vec<&str> = rest.iter().map(|segment| segment.name.as_str()).collect();
        let rest = rest.join(".");

        if !rest.is_empty() && self.is_published(&first.name, &rest) {
            return self.lookup(&first.name, &rest);
        }

        None
    }

    /// Resolves a name that must be one of `kinds`, reporting it otherwise.
    pub(super) fn expect_kind(
        &mut self,
        scope: &Scope,
        path: &Path,
        kinds: &[SymbolKind],
        expected: &str,
    ) -> Option<Resolved> {
        let text = path.text();

        match self.resolve(scope, path) {
            None => {
                let message = crate::standard::not_found_message(expected, &text);
                self.error(message, path.span());
                None
            }
            Some(found) if !kinds.contains(&found.kind) => {
                let actual = found.kind.describe();

                self.error(
                    format!("expected {expected}, but `{text}` is {actual}"),
                    path.span(),
                );

                None
            }
            Some(found) => Some(found),
        }
    }
}
