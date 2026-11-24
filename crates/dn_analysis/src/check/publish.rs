//! What a context publishes, and what it imports from others.

use super::{Checker, Context};

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

    /// `use { ... } from X` imports only what context `X` publishes.
    pub(super) fn check_uses(&mut self, context: &Context) {
        for use_decl in &context.uses {
            let from = &use_decl.from;
            if from.name == context.name.name {
                self.error(
                    format!("context `{}` cannot import from itself", from.name),
                    from.span,
                );
                continue;
            }
            if !self.program.contexts.contains_key(&from.name) {
                self.error(
                    format!("cannot find a context named `{}`", from.name),
                    from.span,
                );
                continue;
            }
            for path in &use_decl.names {
                let text = path.text();
                if !self.is_published(&from.name, &text) || self.lookup(&from.name, &text).is_none()
                {
                    let message = format!("context `{}` does not publish `{text}`", from.name);
                    self.error(message, path.span());
                }
            }
        }
    }
}
