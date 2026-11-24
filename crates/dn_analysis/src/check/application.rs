//! The application: its starting service, and an adapter for every port each
//! target needs.

use std::collections::{BTreeSet, HashSet};

use dn_syntax::ast::{ApplicationDecl, TargetDecl};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    pub(super) fn check_application(&mut self, application: &ApplicationDecl) {
        self.find_anywhere(&application.start, SymbolKind::Service, "a service");
        let needed = self.ports_used_by_services();
        let mut targets = HashSet::new();
        for target in &application.targets {
            if !targets.insert(target.name.name.as_str()) {
                let message = format!("target `{}` is declared more than once", target.name.name);
                self.error(message, target.name.span);
            }
            self.check_target(target, &needed);
        }
    }

    fn check_target(&mut self, target: &TargetDecl, needed: &BTreeSet<(String, String)>) {
        let mut bound = BTreeSet::new();
        for (port_path, adapter_path) in &target.bindings {
            let Some(port) = self.find_anywhere(port_path, SymbolKind::Port, "a port") else {
                continue;
            };
            if !bound.insert((port.context.clone(), port.name.clone())) {
                let message = format!("port `{}` already has an adapter on this target", port.name);
                self.error(message, port_path.span());
            }
            let Some(adapter) = self.find_anywhere(adapter_path, SymbolKind::Adapter, "an adapter")
            else {
                continue;
            };
            let context = &self.program.contexts[&adapter.context];
            let decl = context
                .adapters
                .iter()
                .find(|a| a.name.name == adapter.name)
                .expect("an adapter");
            let scope = Scope {
                context,
                aggregate: None,
            };
            if self.resolve(&scope, &decl.port).as_ref() != Some(&port) {
                let message = format!("adapter `{}` is not for port `{}`", adapter.name, port.name);
                self.error(message, adapter_path.span());
            }
            if decl.target.name != target.name.name {
                let message = format!(
                    "adapter `{}` is for target `{}`, not `{}`",
                    adapter.name, decl.target.name, target.name.name
                );
                self.error(message, adapter_path.span());
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

    /// Every port a service uses, directly or through a repository.
    fn ports_used_by_services(&self) -> BTreeSet<(String, String)> {
        let mut ports = BTreeSet::new();
        for context in self.program.contexts.values() {
            let scope = Scope {
                context,
                aggregate: None,
            };
            let services = context.functions.iter().flat_map(|function| &function.uses);
            for path in services {
                let Some(found) = self.resolve(&scope, path) else {
                    continue;
                };
                let storage = match found.kind {
                    SymbolKind::Port => Some(found),
                    SymbolKind::Repository => context
                        .repositories
                        .iter()
                        .find(|repository| repository.name.name == found.name)
                        .and_then(|repository| repository.storage.as_ref())
                        .and_then(|storage| self.resolve(&scope, storage)),
                    _ => None,
                };
                if let Some(port) = storage.filter(|port| port.kind == SymbolKind::Port) {
                    ports.insert((port.context, port.name));
                }
            }
        }
        ports
    }
}
