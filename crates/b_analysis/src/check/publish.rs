//! What a context publishes, and what it imports from others.

use b_syntax::Span;

use super::{Checker, Context};
use crate::standard::{standard_context_of, standard_names};

impl<'a> Checker<'a> {
    /// A context can only publish names it declares itself.
    pub(super) fn check_publish(&mut self, context: &Context) {
        let name = &context.name.name;
        for path in &context.publish {
            if self.lookup(name, &path.text()).is_none() {
                let message = format!(
                    "cannot publish `{}`: context `{name}` does not declare it",
                    path.text()
                );
                self.error(message, path.span());
            }
        }
    }

    /// `use { ... } from X` imports only what context `X` publishes, or what
    /// a standard context such as `Std.Text` provides.
    pub(super) fn check_uses(&mut self, context: &Context) {
        for use_decl in &context.uses {
            let from = use_decl.from.text();
            let span = use_decl.from.span();
            if let Some(provided) = standard_names(&from) {
                for path in &use_decl.names {
                    if !provided.contains(&path.text().as_str()) {
                        let message = format!("`{from}` does not provide `{}`", path.text());
                        self.error(message, path.span());
                    }
                }
                continue;
            }
            if from == context.name.name {
                self.error(format!("context `{from}` cannot import from itself"), span);
                continue;
            }
            if !self.program.contexts.contains_key(&from) {
                self.error(format!("cannot find a context named `{from}`"), span);
                continue;
            }
            for path in &use_decl.names {
                let text = path.text();
                if !self.is_published(&from, &text) || self.lookup(&from, &text).is_none() {
                    let message = format!("context `{from}` does not publish `{text}`");
                    self.error(message, path.span());
                }
            }
        }
    }

    /// Whether `context` may use the built-in `name`: either it needs no
    /// import, or the context imports it from its standard context. Reports
    /// the import it needs when it may not.
    pub(super) fn check_standard(&mut self, context: &Context, name: &str, span: Span) -> bool {
        let Some(standard) = standard_context_of(name) else {
            return true;
        };
        let imported = context.uses.iter().any(|use_decl| {
            use_decl.from.text() == standard && use_decl.names.iter().any(|n| n.text() == name)
        });
        if !imported {
            let message = format!("`{name}` needs `use {{ {name} }} from {standard}`");
            self.error(message, span);
        }
        imported
    }
}
