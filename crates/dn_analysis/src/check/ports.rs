//! The ports a program's services use, directly or through a repository.

use std::collections::BTreeSet;

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// Every port a service calls, directly or through a repository.
    pub(super) fn ports_used_by_services(&self) -> BTreeSet<(String, String)> {
        let mut ports = BTreeSet::new();
        for context in self.program.contexts.values() {
            let scope = Scope {
                context,
                aggregate: None,
            };
            let services = context
                .functions
                .iter()
                .flat_map(|function| &function.calls);
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
