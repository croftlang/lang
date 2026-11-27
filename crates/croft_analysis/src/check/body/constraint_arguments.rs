//! The arguments of the constraints on parameters. One can name another
//! parameter, `high: Int where AtLeast(low)`, so each is checked as an
//! expression with the parameters in scope, like a contract line.

use croft_syntax::ast::{ConstraintUse, Expr, Parameter};

use super::known_constraints::names_in;
use super::BodyChecker;
use crate::check::constraints::whole_literal_past_32_bits;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks the arguments of each parameter's constraints against the
    /// types the constraint takes. The parameters must be bound already.
    pub(super) fn check_constraint_arguments(&mut self, parameters: &[Parameter]) {
        for parameter in parameters {
            let ty = self.checker.parameter_type(&self.scope, parameter);

            for constraint in &parameter.constraints {
                self.check_arguments_of(constraint, &ty);
            }
        }
    }

    /// Checks the arguments of `constraint`, used on a value of type `ty`.
    pub(in crate::check) fn check_arguments_of(&mut self, constraint: &ConstraintUse, ty: &Type) {
        let checked = match (constraint.each, ty) {
            (true, Type::List(item)) => item.as_ref().clone(),
            _ => ty.clone(),
        };

        for (index, argument) in constraint.arguments.iter().enumerate() {
            // `check_constraint_use` reports a whole number past 32 bits already.
            if whole_literal_past_32_bits(argument).is_some() {
                continue;
            }

            match self.constraint_argument_type(constraint, index, &checked) {
                Some(expected) => self.expect_constraint_argument(argument, &expected),
                None => {
                    self.expr(argument, None);
                }
            }
        }
    }

    /// Checks the arguments of a constraint on a function's result, which
    /// can read constants but not the function's `parameters`.
    pub(super) fn check_result_constraint_arguments(
        &mut self,
        constraint: &ConstraintUse,
        ty: &Type,
        parameters: &[Parameter],
    ) {
        let mut names = Vec::new();

        for argument in &constraint.arguments {
            names_in(argument, &mut names);
        }

        let read = names
            .iter()
            .find(|name| parameters.iter().any(|p| p.name.name == **name));

        if let Some(name) = read {
            let message = format!(
                "`{}` on the result reads the parameter `{name}`, which only a parameter's constraint can",
                constraint.text()
            );

            self.error(message, constraint.name.span());
            return;
        }

        self.check_arguments_of(constraint, ty);
    }

    /// The type the constraint takes for its argument at `index`, after the
    /// value it checks, when one is known: a number for `Range`, of the
    /// value's own type when that is a `Decimal`, and a whole number for
    /// `Length`.
    fn constraint_argument_type(
        &self,
        constraint: &ConstraintUse,
        index: usize,
        checked: &Type,
    ) -> Option<Type> {
        let Some(found) = self.checker.find_path(&self.scope, &constraint.name) else {
            return match constraint.name.text().as_str() {
                "Range" if matches!(checked, Type::Decimal(_)) => Some(checked.clone()),
                "Range" | "Length" => Some(Type::Int),
                _ => None,
            };
        };

        if found.kind != SymbolKind::Constraint {
            return None;
        }

        let owner = self.checker.scope_of(&found);
        let declared = (owner.context.constraints.iter()).find(|c| c.name.name == found.name)?;
        let parameter = declared.parameters.get(index + 1)?;

        match self.checker.parameter_type(&owner, parameter) {
            Type::Unknown => None,
            ty => Some(ty),
        }
    }

    /// Checks one argument against the type its constraint takes. A whole
    /// number is a bound for a `Decimal` too.
    fn expect_constraint_argument(&mut self, argument: &Expr, expected: &Type) {
        let found = self.expr(argument, Some(expected));

        if matches!(expected, Type::Decimal(_)) && found == Type::Int {
            return;
        }

        if !self.accepts(expected, &found, argument) {
            let message = format!("expected `{expected}`, found `{found}`");
            self.error(message, argument.span);
        }
    }
}
