//! Testing a value against a `where` constraint, for debug builds.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::{Argument, ConstraintUse};

use super::Writer;
use crate::names::{c_name, c_string};
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
        let message = c_string(&format!("{what} must satisfy {}", describe(constraint)));

        if constraint.each {
            let Type::List(item) = ty else { return };
            let index = self.variable("index", &Type::Int);

            self.open(format!(
                "for ({index} = 0; {index} < (croft_int) {value}.length; {index}++)"
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
                    ("Length", [exact]) => Some(format!("((croft_int) {value}.length == {exact}L)")),
                    ("Length", [low, high]) => Some(format!(
                        "((croft_int) {value}.length >= {low}L && (croft_int) {value}.length <= {high}L)"
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

/// `Range(1, 50)` or `each Alive`, as written, for messages.
fn describe(constraint: &ConstraintUse) -> String {
    let name = constraint.name.text();
    let arguments: Vec<String> = constraint.arguments.iter().map(describe_argument).collect();

    let call = if arguments.is_empty() {
        name
    } else {
        format!("{name}({})", arguments.join(", "))
    };

    if constraint.each {
        format!("each {call}")
    } else {
        call
    }
}

fn describe_argument(value: &croft_syntax::ast::Expr) -> String {
    use croft_syntax::ast::{BinaryOp, ExprKind, UnaryOp};

    match &value.kind {
        ExprKind::Int(number) => number.to_string(),
        ExprKind::Float(text) => text.clone(),
        ExprKind::Name(name) => name.clone(),
        ExprKind::Unary(UnaryOp::Negate, inner) => format!("-{}", describe_argument(inner)),
        ExprKind::Unary(UnaryOp::BitNot, inner) => format!("~{}", describe_argument(inner)),
        ExprKind::Binary(op, left, right) => {
            let symbol = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
                BinaryOp::BitAnd => "&",
                BinaryOp::BitOr => "|",
                BinaryOp::BitXor => "^",
                BinaryOp::ShiftLeft => "<<",
                BinaryOp::ShiftRight => ">>",
                _ => "?",
            };

            format!(
                "{} {symbol} {}",
                describe_argument(left),
                describe_argument(right)
            )
        }
        _ => "...".to_string(),
    }
}
