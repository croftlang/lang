//! Testing a value against a `where` constraint: when checks are on, and
//! always for a value read back from `Storage`.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::{Argument, ConstraintUse, Expr, ExprKind};

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
    /// when its arguments can't be worked out now. An argument is a constant,
    /// or reads the other parameters bound here: `AtLeast(low)`.
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

        let numbers: Vec<Option<i64>> = (constraint.arguments.iter())
            .map(|argument| match ty {
                Type::Decimal(places) if found.is_none() => {
                    decimal_bound(program, model, context, argument, *places)
                }
                _ => evaluate(program, model, context, argument),
            })
            .collect();

        let reads_parameters: Vec<bool> = (constraint.arguments.iter())
            .map(|argument| self.reads_parameters(argument))
            .collect();

        let known = |index: usize| numbers[index].is_some() || reads_parameters[index];

        if !(0..numbers.len()).all(known) {
            return None;
        }

        match found.filter(|f| f.kind == SymbolKind::Constraint) {
            Some(found) => {
                let rest: Vec<(String, Type)> =
                    self.gen.parameters_of(&found).into_iter().skip(1).collect();

                let arguments: Vec<Argument> = (numbers.iter().zip(&constraint.arguments))
                    .map(|(number, original)| Argument {
                        name: None,
                        value: match number {
                            Some(number) => literal(*number, original),
                            None => original.clone(),
                        },
                    })
                    .collect();

                let mut passed = vec![self.pass(value.to_string(), ty)];
                passed.extend(self.pass_all(&arguments, &rest));

                Some(self.call_into(&c_name(&found), passed, &Type::Bool))
            }
            None => {
                let bound = match ty {
                    Type::Decimal(_) if constraint.name.text() == "Range" => ty.clone(),
                    _ => Type::Int,
                };

                let bounds: Vec<Bound> = (numbers.iter().zip(&constraint.arguments))
                    .map(|(number, argument)| match number {
                        Some(number) => Bound::Known(*number),
                        None => Bound::Read(self.value(argument, &bound)),
                    })
                    .collect();

                match (constraint.name.text().as_str(), bounds.as_slice()) {
                    ("Range", [low, high]) => int_range_test(value, ty, low, high),
                    ("Length", [exact]) => Some(format!("({value}.length == {})", exact.c())),
                    // A length is never below zero, so that end needs no test.
                    ("Length", [Bound::Known(0), high]) => {
                        Some(format!("({value}.length <= {})", high.c()))
                    }
                    ("Length", [low, high]) => Some(format!(
                        "({value}.length >= {} && {value}.length <= {})",
                        low.c(),
                        high.c()
                    )),
                    _ => None,
                }
            }
        }
    }

    /// Whether `argument` reads a name bound here, such as another parameter,
    /// and every name it reads is one, so it can be worked out in C. The
    /// checker gave it a type only then.
    fn reads_parameters(&self, argument: &Expr) -> bool {
        let mut names = Vec::new();
        names_in(argument, &mut names);

        let typed = !matches!(self.gen.facts.type_of(argument.id), Type::Unknown);

        typed && !names.is_empty() && names.iter().all(|name| self.lookup(name).is_some())
    }
}

/// One end of a `Range` or `Length`: a number known now, or C that reads a
/// parameter while the program runs.
enum Bound {
    Known(i64),
    Read(String),
}

impl Bound {
    /// The bound as C, compared with a whole number of any size.
    fn long(&self) -> String {
        match self {
            Bound::Known(number) => format!("{number}L"),
            Bound::Read(c) => c.clone(),
        }
    }

    /// The bound as C, compared with a length.
    fn c(&self) -> String {
        match self {
            Bound::Known(number) => c_number(*number),
            Bound::Read(c) => c.clone(),
        }
    }
}

/// Adds each bare name `value` reads to `names`, as `names_in` does for the
/// checker: the names of the parameters a constraint's argument reads.
fn names_in(value: &Expr, names: &mut Vec<String>) {
    match &value.kind {
        ExprKind::Name(name) => names.push(name.clone()),
        ExprKind::Field(receiver, _) => names_in(receiver, names),
        ExprKind::Unary(_, operand) => names_in(operand, names),
        ExprKind::Binary(_, left, right) => {
            names_in(left, names);
            names_in(right, names);
        }
        ExprKind::Call(callee, arguments) => {
            // A function's own name is not a value it reads.
            if let ExprKind::Field(receiver, _) = &callee.kind {
                names_in(receiver, names);
            }

            for argument in arguments {
                names_in(&argument.value, names);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                names_in(item, names);
            }
        }
        _ => {}
    }
}

/// The C test of `value` against `Range(low, high)`. An `Int` can't go past
/// its own 32 bits, so an end known to be the lowest or highest `Int` needs
/// no test, and C compilers warn that one is always true; `None` when
/// neither end does.
fn int_range_test(value: &str, ty: &Type, low: &Bound, high: &Bound) -> Option<String> {
    let int = matches!(ty, Type::Int);
    let low_test = !(int && matches!(low, Bound::Known(low) if *low <= i64::from(i32::MIN)));
    let high_test = !(int && matches!(high, Bound::Known(high) if *high >= i64::from(i32::MAX)));

    let (low, high) = (low.long(), high.long());

    match (low_test, high_test) {
        (true, true) => Some(format!("({value} >= {low} && {value} <= {high})")),
        (true, false) => Some(format!("({value} >= {low})")),
        (false, true) => Some(format!("({value} <= {high})")),
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
