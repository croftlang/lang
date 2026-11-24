//! The domain: one per program, listing every context it is divided into.

use std::collections::HashSet;

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
        let real = |name: &str| {
            self.program
                .contexts
                .get(name)
                .is_some_and(|c| !c.is_domain)
        };
        if real(&domain.name.name) {
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
            if !listed.insert(context.name.as_str()) {
                self.error(
                    format!("context `{}` is listed more than once", context.name),
                    context.span,
                );
            } else if !real(&context.name) {
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
        for use_decl in &domain.uses_from {
            if real(&use_decl.from.name) && !listed.contains(use_decl.from.name.as_str()) {
                let message = format!(
                    "domain `{}` can only use contexts it lists in `contexts`",
                    domain.name.name
                );
                self.error(message, use_decl.from.span);
            }
        }
    }
}
