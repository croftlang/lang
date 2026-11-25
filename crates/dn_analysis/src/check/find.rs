//! Finding a declaration from outside any context, as the application does.

use dn_syntax::ast::Path;

use super::{Checker, Resolved};
use crate::symbols::SymbolKind;

impl<'a> Checker<'a> {
    /// Finds a declaration in any context. `Dungeon.PlayGame` names the context.
    pub(super) fn find_anywhere(
        &mut self,
        path: &Path,
        kind: SymbolKind,
        expected: &str,
    ) -> Option<Resolved> {
        let text = path.text();
        let mut found: Vec<Resolved> = self
            .tables
            .keys()
            .filter_map(|context| self.lookup(context, &text))
            .collect();
        if let [first, rest @ ..] = &path.segments[..] {
            let rest: Vec<&str> = rest.iter().map(|segment| segment.name.as_str()).collect();
            found.extend(self.lookup(&first.name, &rest.join(".")));
        }
        found.retain(|resolved| resolved.kind == kind);
        match found.len() {
            1 => found.pop(),
            0 => {
                self.error(
                    format!("cannot find {expected} named `{text}`"),
                    path.span(),
                );
                None
            }
            _ => {
                let message = format!(
                    "`{text}` is declared in more than one context; write `Context.{text}`"
                );
                self.error(message, path.span());
                None
            }
        }
    }
}
