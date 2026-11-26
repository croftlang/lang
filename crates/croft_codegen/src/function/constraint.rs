//! Testing a value against a `where` constraint, when checks are on.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::{Argument, ConstraintUse};

use super::range::c_number;
use super::Writer;
use crate::names::{c_name, c_string};
use crate::types::bounds::list_item;
use crate::types::constant::{decimal_bound, evaluate};

impl<'g, 'a> Writer<'g, 'a> {
    /// Writes a check that stops the program when `value` breaks `constraint`.
    /// `what` names the value in the message, such as `heal: amount`.
    pub fn check_constraint(
        &mut self,
        value: &str,
        ty: &Type,
        constraint: &ConstraintUse,
        what: &str,
    ) {
        let message = c_string(&format!("{what} must satisfy {}", constraint.text()));

        if constraint.each {
            let Some(item) = list_item(ty) else { return };
            let index = self.index_variable();
            self.open(format!(
                "for ({index} = 0; {index} < {value}.length; {index}++)"
            ));

            if let Some(test) =
                self.constraint_test(&format!("{value}.items[{index}]"), item, constraint)
            {
                self.line(format!("CROFT_CHECK({test}, {message});"));
            }

            self.close();
            return;
        }

        if let Some(test) = self.constraint_test(value, ty, constraint) {
            self.line(format!("CROFT_CHECK({test}, {message});"));
        }
    }

    /// A C expression that is true when `value` keeps the constraint, or `None`
    /// when its arguments can't be worked out now.
    fn constraint_test(
        &mut self,
        value: &str,
        ty: &Type,
        constraint: &ConstraintUse,
    ) -> Option<String> {
        let constraint = &constraint;
        let aggregate = self.aggregate.clone();

        let found = self
            .gen
            .model
            .resolve(&self.context, aggregate.as_deref(), &constraint.name);

        let (program, model, context) = (self.gen.program, &self.gen.model, &self.context);

        let numbers: Option<Vec<i64>> = (constraint.arguments.iter())
            .map(|argument| match ty {
                Type::Decimal(places) if found.is_none() => {
                    decimal_bound(program, model, context, argument, *places)
                }
                _ => evaluate(program, model, context, argument),
            })
            .collect();

        match found.filter(|f| f.kind == SymbolKind::Constraint) {
            Some(found) => {
                let rest: Vec<(String, Type)> =
                    self.gen.parameters_of(&found).into_iter().skip(1).collect();

                let arguments: Vec<Argument> = (numbers?.into_iter().zip(&constraint.arguments))
                    .map(|(number, original)| Argument {
                        name: None,
                        value: literal(number, original),
                    })
                    .collect();

                let mut passed = vec![self.pass(value.to_string(), ty)];
                passed.extend(self.pass_all(&arguments, &rest));

                Some(self.call_into(&c_name(&found), passed, &Type::Bool))
            }
            None => {
                let numbers = numbers?;

                match (constraint.name.text().as_str(), numbers.as_slice()) {
                    ("Range", [low, high]) => {
                        Some(format!("({value} >= {low}L && {value} <= {high}L)"))
                    }
                    ("Length", [exact]) => {
                        Some(format!("({value}.length == {})", c_number(*exact)))
                    }
                    // A length is never below zero, so that end needs no test.
                    ("Length", [0, high]) => {
                        Some(format!("({value}.length <= {})", c_number(*high)))
                    }
                    ("Length", [low, high]) => Some(format!(
                        "({value}.length >= {} && {value}.length <= {})",
                        c_number(*low),
                        c_number(*high)
                    )),
                    _ => None,
                }
            }
        }
    }
}

/// A constant argument as a literal, so the checker's notes aren't needed.
fn literal(number: i64, original: &croft_syntax::ast::Expr) -> croft_syntax::ast::Expr {
    croft_syntax::ast::Expr {
        kind: croft_syntax::ast::ExprKind::Int(number),
        span: original.span,
        id: original.id,
    }
}
