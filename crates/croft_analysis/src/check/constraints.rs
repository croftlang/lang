//! `where` clauses and the constraints they apply.

use croft_syntax::ast::{ConstraintUse, TypeExpr};

use super::{Checker, Scope};
use crate::facts::BuiltinName;
use crate::symbols::SymbolKind;

/// How many arguments each built-in constraint takes, after the value it checks.
const BUILTIN_CONSTRAINTS: [(&str, &[usize]); 2] = [("Range", &[2]), ("Length", &[1, 2])];

/// Where a `where` clause is written, which decides whether its constraints
/// may end with `or Error`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConstraintPlace {
    /// A parameter of a function, service, command or factory: the call can
    /// fail with the error.
    Parameter,
    /// A parameter of a port operation or of a constraint, which has no body
    /// of its own to fail.
    Declaration,
    Field,
    Result,
    Identity,
    LoopCounter,
}

impl ConstraintPlace {
    /// Why a constraint here cannot fail with an error, or `None` when it can.
    fn without_errors(self) -> Option<&'static str> {
        match self {
            ConstraintPlace::Parameter => None,
            ConstraintPlace::Declaration => {
                Some("this parameter belongs to a declaration with no body to fail")
            }
            ConstraintPlace::Field => Some("a field is part of a value that is always made"),
            ConstraintPlace::Result => Some("a result is given back by the function itself"),
            ConstraintPlace::Identity => Some("an identity is always made"),
            ConstraintPlace::LoopCounter => {
                Some("a loop counter only takes the values its constraints allow")
            }
        }
    }
}

impl<'a> Checker<'a> {
    /// Checks the constraints of a `where` clause: `each` only on a list, and
    /// `or Error` only where `place` allows it.
    pub(super) fn check_constraints(
        &mut self,
        scope: &Scope,
        constraints: &[ConstraintUse],
        ty: Option<&TypeExpr>,
        place: ConstraintPlace,
    ) {
        let is_list = ty.is_some_and(|ty| ty.path.text() == "List");

        for constraint in constraints {
            if constraint.each && !is_list {
                self.error("`each` can only be used on a list", constraint.name.span());
            }

            self.check_constraint_use(scope, constraint);
        }

        self.reject_constraint_errors(constraints, place);
    }

    /// Reports each `or Error` on a constraint written where nothing can fail.
    pub(super) fn reject_constraint_errors(
        &mut self,
        constraints: &[ConstraintUse],
        place: ConstraintPlace,
    ) {
        let Some(reason) = place.without_errors() else {
            return;
        };

        for constraint in constraints {
            let Some(error) = &constraint.error else {
                continue;
            };

            let message = format!(
                "only a parameter's constraint can fail with an error: {reason}, so `{}` must hold",
                constraint.text()
            );

            self.error(message, error.span);
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

                self.facts.builtins.push(BuiltinName {
                    span: used.name.span(),
                    name: name.clone(),
                });

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

    /// How many parameters the constraint `name` in `context` declares, or 0
    /// when there is no such constraint.
    fn constraint_parameter_count(&self, context: &str, name: &str) -> usize {
        let constraints = &self.program.contexts[context].constraints;

        let constraint = constraints
            .iter()
            .find(|constraint| constraint.name.name == name);

        constraint.map_or(0, |constraint| constraint.parameters.len())
    }
}
