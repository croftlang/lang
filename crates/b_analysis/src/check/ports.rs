//! The ports a program's services call.

use std::collections::BTreeSet;

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// Every port a service calls. A repository's storage is chosen in b.toml.
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
                if found.kind == SymbolKind::Port {
                    ports.insert((found.context, found.name));
                }
            }
        }
        ports
    }
}
