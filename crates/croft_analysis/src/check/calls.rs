//! The ports a service or domain lists with `calls { ... }`, perhaps from
//! another context and perhaps under another name.

use std::collections::BTreeSet;

use croft_syntax::ast::{CalledPort, FunctionDecl, FunctionKind};

use super::{Checker, Scope};
use crate::standard::{standard_context_publishing, standard_names};
use crate::symbols::SymbolKind;
use crate::types::Resolved;

impl<'a> Checker<'a> {
    /// What a `calls` entry names: looked up in the context after `from`
    /// when there is one, otherwise from where the service is.
    pub(super) fn find_called(&self, scope: &Scope, called: &CalledPort) -> Option<Resolved> {
        let path = &called.name.path;

        let Some(from) = &called.from else {
            return self.find_path(scope, path);
        };

        let (from, text) = (from.text(), path.text());
        let provided = standard_names(&from).is_some_and(|names| names.contains(&text.as_str()));

        if !provided && !self.is_published(&from, &text) {
            return None;
        }

        self.lookup(&from, &text)
    }

    /// Only a service or the domain calls ports, each a port or repository,
    /// each listed once under the name the body calls it by.
    pub(super) fn check_calls(&mut self, scope: &Scope, function: &FunctionDecl) {
        if !matches!(function.kind, FunctionKind::Service | FunctionKind::Domain) {
            if let Some(called) = function.calls.first() {
                self.error("only a service can call ports", called.name.path.span());
            }

            return;
        }

        let mut listed = BTreeSet::new();

        for called in &function.calls {
            self.check_called(scope, called);

            let local = called.name.local_name();
            let span = match &called.name.alias {
                Some(alias) => alias.span,
                None => called.name.path.span(),
            };

            if !listed.insert(local.clone()) {
                self.error(format!("`{local}` is already in `calls`"), span);
            }
        }
    }

    /// One `calls` entry names a port or repository, which a context after
    /// `from` must publish.
    fn check_called(&mut self, scope: &Scope, called: &CalledPort) {
        let path = &called.name.path;
        let text = path.text();

        let found = match &called.from {
            Some(from) => {
                self.check_import(scope.context, from, &[&called.name]);
                self.find_called(scope, called)
            }
            None => {
                let found = self.resolve(scope, path);

                if found.is_none() {
                    let message = match standard_context_publishing(&text) {
                        Some(context) => {
                            format!("`{text}` needs `calls {{ {text} }} from {context}`")
                        }
                        None => format!("cannot find a port or repository named `{text}`"),
                    };

                    self.error(message, path.span());
                }

                self.check_alias(scope, &called.name, found.as_ref());
                found
            }
        };

        let Some(found) = found else {
            return;
        };

        if !matches!(found.kind, SymbolKind::Port | SymbolKind::Repository) {
            let actual = found.kind.describe();
            let message = format!("expected a port or repository, but `{text}` is {actual}");
            self.error(message, path.span());
        }
    }
}
