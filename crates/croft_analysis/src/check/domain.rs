//! The domain: one per program, and its context map. Every context is in the
//! map, a context only uses contexts upstream of it, and the arrows form no
//! cycle.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use croft_syntax::ast::{DomainDecl, Path};

use super::Checker;

impl<'a> Checker<'a> {
    /// There is one domain, not named after a context. Its `contexts` lists
    /// every context, and it uses only contexts it lists.
    pub(super) fn check_domain(&mut self) {
        let Some((domain, others)) = self.program.domains.split_first() else {
            return;
        };

        for other in others {
            let message = format!(
                "a program has one domain, and `{}` is already declared",
                domain.name.name
            );

            self.error(message, other.name.span);
        }

        if self.is_context(&domain.name.name) {
            self.error(
                format!(
                    "domain `{}` has the same name as a context",
                    domain.name.name
                ),
                domain.name.span,
            );
        }

        let mut listed = HashSet::new();

        for context in &domain.contexts {
            if listed.insert(context.name.as_str()) && !self.is_context(&context.name) {
                self.error(
                    format!("cannot find a context named `{}`", context.name),
                    context.span,
                );
            }
        }

        let missing: Vec<String> = (self.program.contexts.values())
            .filter(|c| !c.is_domain && !c.is_standard && !listed.contains(c.name.name.as_str()))
            .map(|c| c.name.name.clone())
            .collect();

        for name in missing {
            let message = format!(
                "context `{name}` is not in the `contexts` of domain `{}`",
                domain.name.name
            );

            self.error(message, domain.name.span);
        }

        for from in domain.uses.iter().map(|use_decl| &use_decl.from).chain(
            domain
                .calls
                .iter()
                .filter_map(|called| called.from.as_ref()),
        ) {
            if self.is_context(&from.text()) && !listed.contains(from.text().as_str()) {
                let message = format!(
                    "domain `{}` can only use contexts it lists in `contexts`",
                    domain.name.name
                );

                self.error(message, from.span());
            }
        }

        self.check_relations(domain);
    }

    /// Whether `name` is a context the program declares, rather than the domain
    /// or a standard context.
    fn is_context(&self, name: &str) -> bool {
        self.program
            .contexts
            .get(name)
            .is_some_and(|context| !context.is_domain && !context.is_standard)
    }

    /// A context may only `use` contexts directly upstream of it.
    fn check_relations(&mut self, domain: &DomainDecl) {
        let mut upstream: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();

        for (from, to) in &domain.relations {
            if from.name == to.name {
                self.error(
                    format!("context `{}` cannot be upstream of itself", from.name),
                    to.span,
                );
            }

            upstream
                .entry(to.name.as_str())
                .or_default()
                .insert(from.name.as_str());
        }

        if let Some(context) = first_in_cycle(&upstream) {
            let message =
                format!("the arrows in `contexts` go round in a circle through `{context}`");
            self.error(message, domain.name.span);
        }

        // Where each context imports from: its `use`s, and the `from` of
        // each port its services call.
        let imports: Vec<(String, &Path)> = (self.program.contexts.values())
            .filter(|c| !c.is_domain)
            .flat_map(|c| {
                let uses = c.uses.iter().map(|use_decl| &use_decl.from);
                let calls = (c.functions.iter())
                    .flat_map(|function| &function.calls)
                    .filter_map(|called| called.from.as_ref());

                uses.chain(calls)
                    .map(move |from| (c.name.name.clone(), from))
            })
            .collect();

        for (context, from_path) in imports {
            let from = from_path.text();
            let from = from.as_str();

            let allowed = upstream
                .get(context.as_str())
                .is_some_and(|set| set.contains(from));

            if self.is_context(from) && !allowed {
                let message =
                    format!("`{context}` uses `{from}`, so `contexts` needs `{from} -> {context}`");
                self.error(message, from_path.span());
            }
        }
    }
}

/// A context the arrows lead back to, if any do.
fn first_in_cycle<'n>(upstream: &BTreeMap<&'n str, BTreeSet<&'n str>>) -> Option<&'n str> {
    /// Follows the arrows upstream from `node`, giving back the first context
    /// reached twice on the current path.
    fn visit<'n>(
        node: &'n str,
        upstream: &BTreeMap<&'n str, BTreeSet<&'n str>>,
        path: &mut Vec<&'n str>,
        done: &mut HashSet<&'n str>,
    ) -> Option<&'n str> {
        if path.contains(&node) {
            return Some(node);
        }

        if !done.insert(node) {
            return None;
        }

        path.push(node);

        for next in upstream.get(node).into_iter().flatten() {
            if let Some(found) = visit(next, upstream, path, done) {
                return Some(found);
            }
        }

        path.pop();

        None
    }

    let mut done = HashSet::new();

    upstream
        .keys()
        .find_map(|node| visit(node, upstream, &mut Vec::new(), &mut done))
}
