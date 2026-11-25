//! The names each context declares, and what kind of thing each one is.
//!
//! Members of an aggregate are listed under the aggregate's name, so the
//! `Enemy` entity of `Room` is `Room.Enemy`. `Room.Error` names all of the
//! aggregate's errors at once.

mod build;
mod kind;

use std::collections::HashMap;

use croft_syntax::ast::Identifier;
use croft_syntax::{Diagnostic, Span};

pub use build::build_table;
pub use kind::SymbolKind;

#[derive(Debug, Clone, Copy)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn get(&self, name: &str) -> Option<Symbol> {
        self.symbols.get(name).copied()
    }

    /// Adds a name, reporting it if the context already declares it.
    fn add(&mut self, name: String, kind: SymbolKind, span: Span, errors: &mut Vec<Diagnostic>) {
        if self.symbols.contains_key(&name) {
            errors.push(Diagnostic::new(
                format!("`{name}` is declared more than once"),
                span,
            ));

            return;
        }

        self.symbols.insert(name, Symbol { kind, span });
    }

    fn add_all<'a>(
        &mut self,
        names: impl IntoIterator<Item = &'a Identifier>,
        prefix: &str,
        kind: SymbolKind,
        errors: &mut Vec<Diagnostic>,
    ) {
        for name in names {
            self.add(format!("{prefix}{}", name.name), kind, name.span, errors);
        }
    }
}
