//! `where` rules and the constraints they apply.

use dn_syntax::ast::{ConstraintUse, Rule, TypeExpr};

use super::{Checker, Scope};
use crate::symbols::SymbolKind;

/// How many arguments each built-in constraint takes, after the value it checks.
const BUILTIN_CONSTRAINTS: [(&str, &[usize]); 2] = [("Range", &[2]), ("Length", &[1, 2])];

impl<'a> Checker<'a> {
    pub(super) fn check_rules(&mut self, scope: &Scope, rules: &[Rule], ty: Option<&TypeExpr>) {
        let is_list = ty.is_some_and(|ty| ty.path.text() == "List");
        for rule in rules {
            if rule.each && !is_list {
                self.error(
                    "`each` can only be used on a list",
                    rule.constraint.name.span(),
                );
            }
            self.check_constraint_use(scope, &rule.constraint);
        }
    }

    /// A constraint's first parameter is the value it checks, so a use has one
    /// argument fewer than the constraint has parameters.
    pub(super) fn check_constraint_use(&mut self, scope: &Scope, used: &ConstraintUse) {
        let name = used.name.text();
        let expected: Vec<usize> = match self.resolve(scope, &used.name) {
            Some(found) if found.kind == SymbolKind::Constraint => {
                let parameters = self.constraint_parameter_count(&found.context, &found.name);
                vec![parameters.saturating_sub(1)]
            }
            None if BUILTIN_CONSTRAINTS
                .iter()
                .any(|(builtin, _)| *builtin == name) =>
            {
                let (_, counts) = BUILTIN_CONSTRAINTS
                    .iter()
                    .find(|(b, _)| *b == name)
                    .unwrap();
                counts.to_vec()
            }
            _ => {
                let kinds = [SymbolKind::Constraint];
                self.expect_kind(scope, &used.name, &kinds, "a constraint");
                return;
            }
        };
        let found = used.arguments.len();
        if !expected.contains(&found) {
            let expected: Vec<String> = expected.iter().map(|count| count.to_string()).collect();
            let expected = expected.join(" or ");
            let message = format!("`{name}` takes {expected} arguments here, found {found}");
            self.error(message, used.name.span());
        }
    }

    fn constraint_parameter_count(&self, context: &str, name: &str) -> usize {
        let constraints = &self.program.contexts[context].constraints;
        let constraint = constraints
            .iter()
            .find(|constraint| constraint.name.name == name);
        constraint.map_or(0, |constraint| constraint.parameters.len())
    }
}
