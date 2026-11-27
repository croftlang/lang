//! Testing a value against a `where` constraint: when checks are on, and
//! always for a value read back from `Storage`.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::{Argument, ConstraintUse, Expr};

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

        self.when_broken(value, ty, constraint, |writer, test| {
            writer.line(format!("CROFT_CHECK({test}, {message});"));
        });
    }

    /// Writes a test that makes the function give back 0 when `value`
    /// breaks `constraint`, as reading a saved value does: what is read is
    /// outside the program, so this is written whatever the target's
    /// `checks` setting.
    pub fn require_constraint(&mut self, value: &str, ty: &Type, constraint: &ConstraintUse) {
        self.when_broken(value, ty, constraint, |writer, test| {
            writer.open(format!("if (!{test})"));
            writer.line("return 0;");
            writer.close();
        });
    }

    /// Calls `broken` with the C test of `value` against `constraint`, or of
    /// each of its items in a loop for an `each` constraint, so it can write
    /// what happens when the test is false.
    fn when_broken(
        &mut self,
        value: &str,
        ty: &Type,
        constraint: &ConstraintUse,
        broken: impl Fn(&mut Self, &str),
    ) {
        if constraint.each {
            let Some(item) = list_item(ty) else { return };
            let index = self.index_variable();
            self.open(format!(
                "for ({index} = 0; {index} < {value}.length; {index}++)"
            ));

            if let Some(test) =
                self.constraint_test(&format!("{value}.items[{index}]"), item, constraint)
            {
                broken(self, &test);
            }

            self.close();
            return;
        }

        if let Some(test) = self.constraint_test(value, ty, constraint) {
            broken(self, &test);
        }
    }

    /// Writes, for each parameter constraint with `or`, a test that fails the
    /// call with its error when the constraint is broken. These are what the
    /// program does, not checks, so they are written whatever the target's
    /// `checks` setting.
    pub fn constraint_failures(&mut self, parameters: &[(String, Type, &[ConstraintUse])]) {
        for (parameter, ty, constraints) in parameters {
            for constraint in constraints.iter() {
                let Some(error) = &constraint.error else {
                    continue;
                };

                let Some((value, _)) = self.lookup(parameter) else {
                    continue;
                };

                if !constraint.each {
                    let Some(test) = self.constraint_test(&value, ty, constraint) else {
                        continue;
                    };

                    self.fail_unless(&test, error);
                    continue;
                }

                let Some(item) = list_item(ty) else { continue };
                let index = self.index_variable();
                self.open(format!(
                    "for ({index} = 0; {index} < {value}.length; {index}++)"
                ));

                if let Some(test) =
                    self.constraint_test(&format!("{value}.items[{index}]"), item, constraint)
                {
                    self.fail_unless(&test, error);
                }

                self.close();
            }
        }
    }

    /// `if (!test)`, failing with `error`.
    fn fail_unless(&mut self, test: &str, error: &Expr) {
        self.open(format!("if (!{test})"));

        let error = self.value(error, &Type::Errors(Vec::new()));
        self.fail(&error);
        self.close();
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
                    ("Range", [low, high]) => int_range_test(value, ty, *low, *high),
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

/// The C test of `value` against `Range(low, high)`. An `Int` can't go past
/// its own 32 bits, so an end at the lowest or highest `Int` needs no test,
/// and C compilers warn that one is always true; `None` when neither end
/// does.
fn int_range_test(value: &str, ty: &Type, low: i64, high: i64) -> Option<String> {
    let int = matches!(ty, Type::Int);
    let low_test = !(int && low <= i64::from(i32::MIN));
    let high_test = !(int && high >= i64::from(i32::MAX));

    match (low_test, high_test) {
        (true, true) => Some(format!("({value} >= {low}L && {value} <= {high}L)")),
        (true, false) => Some(format!("({value} >= {low}L)")),
        (false, true) => Some(format!("({value} <= {high}L)")),
        (false, false) => None,
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
