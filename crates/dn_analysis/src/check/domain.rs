//! The domain: one per program, and its context map. Every context is in the
//! map, a context only uses contexts upstream of it, and the arrows form no
//! cycle.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use dn_syntax::ast::DomainDecl;

use super::Checker;

impl<'a> Checker<'a> {
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
            .filter(|c| !c.is_domain && !listed.contains(c.name.name.as_str()))
            .map(|c| c.name.name.clone())
            .collect();
        for name in missing {
            let message = format!(
                "context `{name}` is not in the `contexts` of domain `{}`",
                domain.name.name
            );
            self.error(message, domain.name.span);
        }
        for use_decl in &domain.uses {
            if self.is_context(&use_decl.from.name) && !listed.contains(use_decl.from.name.as_str())
            {
                let message = format!(
                    "domain `{}` can only use contexts it lists in `contexts`",
                    domain.name.name
                );
                self.error(message, use_decl.from.span);
            }
        }
        self.check_relations(domain);
    }

    fn is_context(&self, name: &str) -> bool {
        self.program
            .contexts
            .get(name)
            .is_some_and(|context| !context.is_domain)
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
        let uses: Vec<_> = (self.program.contexts.values())
            .filter(|c| !c.is_domain)
            .flat_map(|c| {
                c.uses
                    .iter()
                    .map(move |use_decl| (c.name.name.clone(), use_decl))
            })
            .collect();
        for (context, use_decl) in uses {
            let from = use_decl.from.name.as_str();
            let allowed = upstream
                .get(context.as_str())
                .is_some_and(|set| set.contains(from));
            if self.is_context(from) && !allowed {
                let message =
                    format!("`{context}` uses `{from}`, so `contexts` needs `{from} -> {context}`");
                self.error(message, use_decl.from.span);
            }
        }
    }
}

/// A context the arrows lead back to, if any do.
fn first_in_cycle<'n>(upstream: &BTreeMap<&'n str, BTreeSet<&'n str>>) -> Option<&'n str> {
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
