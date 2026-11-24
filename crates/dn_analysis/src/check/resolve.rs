//! Finding what a name refers to.
//!
//! A name is looked up in the current aggregate, then the context, then the
//! names the context imports with `use`. A name can also be written with its
//! context in front, `Dungeon.Room.EnemyDefeated`, if that context publishes it.

use dn_syntax::ast::Path;

use super::Checker;
use crate::program::Context;
use crate::symbols::SymbolKind;

/// Where a name is being looked up from.
#[derive(Clone, Copy)]
pub(crate) struct Scope<'a> {
    pub context: &'a Context,
    pub aggregate: Option<&'a str>,
}

/// A declaration a name refers to: its context, its full name there, and its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub context: String,
    pub name: String,
    pub kind: SymbolKind,
}

impl<'a> Checker<'a> {
    pub(super) fn lookup(&self, context: &str, name: &str) -> Option<Resolved> {
        let symbol = self.tables.get(context)?.get(name)?;
        let (context, name) = (context.to_string(), name.to_string());
        Some(Resolved {
            context,
            name,
            kind: symbol.kind,
        })
    }

    pub(super) fn is_published(&self, context: &str, name: &str) -> bool {
        let Some(context) = self.program.contexts.get(context) else {
            return false;
        };
        context.publish.iter().any(|path| path.text() == name)
    }

    pub(super) fn resolve(&self, scope: &Scope, path: &Path) -> Option<Resolved> {
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
            if use_decl.names.iter().any(|name| name.text() == text) {
                return self.lookup(&use_decl.from.name, &text);
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
                self.error(
                    format!("cannot find {expected} named `{text}`"),
                    path.span(),
                );
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
