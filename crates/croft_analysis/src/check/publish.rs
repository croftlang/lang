//! What a context publishes, and what it imports from others.

use croft_syntax::ast::{ImportedName, Path};
use croft_syntax::Span;

use super::{Checker, Context, Scope};
use crate::standard::{standard_contexts_of, standard_names, SHARED_FUNCTIONS};
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

        self.check_shared_imports(context);
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

            let built_in = standard_contexts_of(&text).any(|standard| standard == from);

            if built_in && target.is_none() {
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
    /// import, or the context imports it from a standard context that
    /// provides it. Reports the import it needs when it may not.
    pub(super) fn check_standard(&mut self, context: &Context, name: &str, span: Span) -> bool {
        let Some(standard) = standard_contexts_of(name).next() else {
            return true;
        };

        if imported_standard(context, name).is_some() {
            return true;
        }

        self.report_missing_import(standard, name, span);

        false
    }

    /// Whether `context` imports the built-in `name` from `standard`, the one
    /// standard context whose `name` fits where it is used, such as
    /// `Std.Encoding` for `to_hex` on `Bytes`. Reports the import it needs
    /// when it doesn't.
    pub(super) fn check_standard_from(
        &mut self,
        context: &Context,
        standard: &str,
        name: &str,
        span: Span,
    ) -> bool {
        if imports(context, standard, name) {
            return true;
        }

        self.report_missing_import(standard, name, span);

        false
    }

    /// Reports that the function `name`, which more than one standard
    /// context provides, needs importing from `standard`, or from any of
    /// them when it isn't known which.
    pub(super) fn report_missing_shared_import(
        &mut self,
        name: &str,
        standard: Option<&str>,
        span: Span,
    ) {
        if let Some(standard) = standard {
            self.report_missing_import(standard, name, span);
            return;
        }

        let imports: Vec<String> = standard_contexts_of(name)
            .map(|standard| format!("`use {{ {name} }} from {standard}`"))
            .collect();

        let message = format!("`{name}` needs {}", imports.join(" or "));
        self.error(message, span);
    }

    /// A function two standard contexts provide, such as `from_hex`, means
    /// what the context imports, so it can't be imported from both.
    fn check_shared_imports(&mut self, context: &Context) {
        for name in SHARED_FUNCTIONS {
            let imported: Vec<(&Path, &ImportedName)> = (context.uses.iter())
                .filter(|use_decl| {
                    standard_names(&use_decl.from.text()).is_some_and(|names| names.contains(&name))
                })
                .flat_map(|use_decl| {
                    (use_decl.names.iter())
                        .filter(|n| n.path.text() == name)
                        .map(move |n| (&use_decl.from, n))
                })
                .collect();

            let Some((first, _)) = imported.first() else {
                continue;
            };

            let first = first.text();
            let other = imported.iter().find(|(from, _)| from.text() != first);

            let Some((second, again)) = other else {
                continue;
            };

            let second = second.text();

            let message = format!(
                "`{name}` can't come from both `{first}` and `{second}`: \
                 they mean different things, so import it from one"
            );

            self.error(message, again.path.span());
        }
    }

    /// Reports that `name` needs importing from `standard`.
    fn report_missing_import(&mut self, standard: &str, name: &str, span: Span) {
        let message = format!("`{name}` needs `use {{ {name} }} from {standard}`");
        self.error(message, span);
    }
}

/// The standard context `context` imports the built-in `name` from, if it
/// imports it from one that provides it.
pub(super) fn imported_standard(context: &Context, name: &str) -> Option<&'static str> {
    standard_contexts_of(name).find(|standard| imports(context, standard, name))
}

/// Whether `context` has `use { name } from standard`.
fn imports(context: &Context, standard: &str, name: &str) -> bool {
    context.uses.iter().any(|use_decl| {
        use_decl.from.text() == standard && use_decl.names.iter().any(|n| n.path.text() == name)
    })
}
