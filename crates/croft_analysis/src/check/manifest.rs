//! Checking `croft.toml` against the program: what each target runs, and an
//! adapter for every port each target needs.

use crate::standard::{is_builtin_port, FILE_STORAGE, STORAGE_CONTEXT};
use std::collections::BTreeSet;

use croft_syntax::ast::Identifier;

use super::{Checker, Resolved, Scope};
use crate::manifest::{to_path, Manifest, TargetConfig};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    pub(super) fn check_manifest(&mut self, manifest: &Manifest) {
        let needed = self.ports_used_by_services();

        for target in &manifest.targets {
            match &target.main {
                Some(main) => self.check_main(main),
                None if self.program.domains.is_empty() => {
                    let message = format!("target `{}` has nothing to run: declare a `domain`, or name a service with `main`", target.name.name);
                    self.error(message, target.name.span);
                }
                None => {}
            }

            self.check_target(target, &needed);
        }
    }

    /// A target's `main` service runs instead of the domain, so it takes no parameters.
    fn check_main(&mut self, main: &Identifier) {
        let Some(service) = self.find_anywhere(&to_path(main), SymbolKind::Service, "a service")
        else {
            return;
        };

        let context = &self.program.contexts[&service.context];

        let decl = context
            .functions
            .iter()
            .find(|f| f.name.name == service.name);

        if decl
            .and_then(|f| f.parameters.as_ref())
            .is_some_and(|p| !p.is_empty())
        {
            self.error(
                format!("the main service `{}` must take no parameters", main.name),
                main.span,
            );
        }
    }

    fn check_target(&mut self, target: &TargetConfig, needed: &BTreeSet<(String, String)>) {
        let mut bound = BTreeSet::new();

        for (port_name, adapter_name) in &target.adapters {
            if let Some(repository) = self.find_quietly(port_name, SymbolKind::Repository) {
                self.check_storage(target, &repository, adapter_name);
                continue;
            }

            if is_builtin_port(&port_name.name)
                && self.find_quietly(port_name, SymbolKind::Port).is_none()
            {
                let message = format!("`{}` is built in, so it needs no adapter", port_name.name);
                self.error(message, port_name.span);
                continue;
            }

            let Some(port) = self.find_anywhere(&to_path(port_name), SymbolKind::Port, "a port")
            else {
                continue;
            };

            if !bound.insert((port.context.clone(), port.name.clone())) {
                let message = format!(
                    "port `{}` already has an adapter on target `{}`",
                    port.name, target.name.name
                );

                self.error(message, port_name.span);
            }

            let Some(adapter) =
                self.find_anywhere(&to_path(adapter_name), SymbolKind::Adapter, "an adapter")
            else {
                continue;
            };

            let context = &self.program.contexts[&adapter.context];

            let decl = context
                .adapters
                .iter()
                .find(|a| a.name.name == adapter.name)
                .expect("a resolved adapter");

            let scope = Scope {
                context,
                aggregate: None,
            };

            if self.resolve(&scope, &decl.port).as_ref() != Some(&port) {
                let message = format!(
                    "adapter `{}` is for port `{}`, not `{}`",
                    adapter.name,
                    decl.port.text(),
                    port.name
                );

                self.error(message, adapter_name.span);
            }
        }

        // A repository the target doesn't bind is kept in `MemoryStorage`.
        for repository in self.all_of(SymbolKind::Repository) {
            let named = (target.adapters.iter()).any(|(port, _)| {
                self.find_quietly(port, SymbolKind::Repository).as_ref() == Some(&repository)
            });

            if !named {
                self.check_storage_shapes(target, &repository, target.name.span);
            }
        }

        for (_, port) in needed.difference(&bound) {
            let message = format!(
                "target `{}` has no adapter for port `{port}`",
                target.name.name
            );

            self.error(message, target.name.span);
        }
    }

    /// A repository is bound to an adapter for `Storage`, such as
    /// `MemoryStorage` or one of the program's own, or to an adapter written
    /// for the repository itself. `FileStorage` saves through whatever
    /// adapter the target binds to `Files`.
    fn check_storage(&mut self, target: &TargetConfig, repository: &Resolved, name: &Identifier) {
        let Some(adapter) = self.find_anywhere(&to_path(name), SymbolKind::Adapter, "an adapter")
        else {
            return;
        };

        let context = &self.program.contexts[&adapter.context];

        let Some(decl) = context
            .adapters
            .iter()
            .find(|a| a.name.name == adapter.name)
        else {
            return;
        };

        let scope = Scope {
            context,
            aggregate: None,
        };

        let port = self.resolve(&scope, &decl.port);

        let is_storage = port
            .as_ref()
            .is_some_and(|p| p.context == STORAGE_CONTEXT && p.name == "Storage");

        if !is_storage && port.as_ref() != Some(repository) {
            let message = format!(
                "`{}` is not an adapter for `Storage` or for `{}`, so it can't keep its aggregates",
                adapter.name, repository.name
            );

            self.error(message, name.span);
            return;
        }

        if is_storage {
            self.check_storage_shapes(target, repository, name.span);
        }

        let saves_in_files = adapter.context == STORAGE_CONTEXT && adapter.name == FILE_STORAGE;
        let binds_files = target.adapters.iter().any(|(port, _)| port.name == "Files");

        if saves_in_files && !binds_files {
            let message = format!(
                "`FileStorage` saves through `Files`, so target `{}` needs an adapter for `Files`, such as `Files = \"StdioFiles\"`",
                target.name.name
            );

            self.error(message, name.span);
        }
    }

    /// `Storage` provides only finds, saves and removes, so a repository
    /// with any other operation needs an adapter written for it.
    fn check_storage_shapes(
        &mut self,
        target: &TargetConfig,
        repository: &Resolved,
        span: croft_syntax::Span,
    ) {
        let Some(aggregate) = self.repository_aggregate(repository) else {
            return;
        };

        for operation in self.operations_of(repository) {
            if self.storage_shape(&aggregate, &operation).is_none() {
                let message = format!(
                    "`Storage` can't provide `{}.{}`, so target `{}` needs an adapter written for `{}`",
                    repository.name, operation.name, target.name.name, repository.name
                );

                self.error(message, span);
            }
        }
    }

    fn find_quietly(&self, name: &Identifier, kind: SymbolKind) -> Option<Resolved> {
        let path = to_path(name);

        self.tables
            .keys()
            .filter_map(|context| self.lookup(context, &path.text()))
            .find(|r| r.kind == kind)
    }
}
