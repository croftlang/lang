//! Checking `dn.toml` against the program: what each target runs, and an
//! adapter for every port each target needs.

use crate::standard::{is_builtin_port, FILE_STORAGE, MEMORY_STORAGE};
use std::collections::BTreeSet;

use dn_syntax::ast::Identifier;

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
            if self
                .find_quietly(port_name, SymbolKind::Repository)
                .is_some()
            {
                self.check_storage(target, adapter_name);
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
        for (_, port) in needed.difference(&bound) {
            let message = format!(
                "target `{}` has no adapter for port `{port}`",
                target.name.name
            );
            self.error(message, target.name.span);
        }
    }

    /// A repository is bound to a built-in storage, and `FileStorage` saves
    /// through whatever adapter the target binds to `Files`.
    fn check_storage(&mut self, target: &TargetConfig, storage: &Identifier) {
        match storage.name.as_str() {
            MEMORY_STORAGE => {}
            FILE_STORAGE => {
                let binds_files = target.adapters.iter().any(|(port, _)| port.name == "Files");
                if !binds_files {
                    let message = format!(
                        "`FileStorage` saves through `Files`, so target `{}` needs an adapter for `Files`, such as `Files = \"StdioFiles\"`",
                        target.name.name
                    );
                    self.error(message, storage.span);
                }
            }
            other => {
                let message = format!(
                    "a repository is kept in `MemoryStorage` or `FileStorage`, not `{other}`"
                );
                self.error(message, storage.span);
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
