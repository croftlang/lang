//! What a context publishes, and what it imports from others.

use croft_syntax::ast::{ImportedName, Path};
use croft_syntax::Span;

use super::{Checker, Context, Scope};
use crate::standard::{standard_context_of, standard_names};
use crate::types::Resolved;

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
            let names: Vec<&ImportedName> = use_decl.names.iter().collect();
            self.check_import(context, &use_decl.from, &names);
        }
    }

    /// Names brought in from `from`, by `use` or by `calls { ... } from`: each
    /// must be published there, or provided by the standard context.
    pub(super) fn check_import(&mut self, context: &Context, from: &Path, names: &[&ImportedName]) {
        let span = from.span();
        let from = from.text();

        self.check_aliases(context, &from, names);

        if let Some(provided) = standard_names(&from) {
            for name in names {
                let text = name.path.text();

                if !provided.contains(&text.as_str()) {
                    let message = format!("`{from}` does not provide `{text}`");
                    self.error(message, name.path.span());
                }
            }

            return;
        }

        if from == context.name.name {
            self.error(format!("context `{from}` cannot import from itself"), span);
            return;
        }

        if !self.program.contexts.contains_key(&from) {
            self.error(format!("cannot find a context named `{from}`"), span);
            return;
        }

        for name in names {
            let path = &name.path;
            let text = path.text();
            let found = self.lookup(&from, &text);

            match found {
                Some(target) if self.is_published(&from, &text) => {
                    let last = path.last();

                    self.references.get_mut().push(crate::facts::Reference {
                        span: last.span,
                        name: last.name.clone(),
                        target,
                    });
                }
                _ => {
                    let message = format!("context `{from}` does not publish `{text}`");
                    self.error(message, path.span());
                }
            }
        }
    }

    /// A name after `as` replaces the declared one, so it can't be one the
    /// context already has. A built-in method or function keeps its own name,
    /// since it is called as written.
    fn check_aliases(&mut self, context: &Context, from: &str, names: &[&ImportedName]) {
        let scope = Scope {
            context,
            aggregate: None,
        };

        for name in names {
            let Some(alias) = &name.alias else {
                continue;
            };

            let text = name.path.text();
            let target = self.lookup(from, &text);

            if standard_context_of(&text) == Some(from) && target.is_none() {
                let message = format!("`{text}` is built in, so it can't be renamed");
                self.error(message, alias.span);
                continue;
            }

            self.check_alias(&scope, name, target.as_ref());
        }
    }

    /// Reports an alias that already names something other than `target` here.
    pub(super) fn check_alias(
        &mut self,
        scope: &Scope,
        name: &ImportedName,
        target: Option<&Resolved>,
    ) {
        let Some(alias) = &name.alias else {
            return;
        };

        let alias_path = Path {
            segments: vec![alias.clone()],
        };

        let taken = self
            .find_path(scope, &alias_path)
            .is_some_and(|found| Some(&found) != target);

        if taken {
            let message = format!(
                "`{}` already names something here, so `{}` can't be called that",
                alias.name,
                name.path.text()
            );

            self.error(message, alias.span);
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
            use_decl.from.text() == standard && use_decl.names.iter().any(|n| n.path.text() == name)
        });

        if !imported {
            let message = format!("`{name}` needs `use {{ {name} }} from {standard}`");
            self.error(message, span);
        }

        imported
    }
}
