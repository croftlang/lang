//! Testing a value against a `where` rule, for debug builds.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;
use dn_syntax::ast::{Argument, Rule};

use super::Writer;
use crate::names::{c_name, c_string};
use crate::types::constant::evaluate;

impl<'g, 'a> Writer<'g, 'a> {
    /// Writes a check that stops the program when `value` breaks `rule`.
    /// `what` names the value in the message, such as `heal: amount`.
    pub fn check_rule(&mut self, value: &str, ty: &Type, rule: &Rule, what: &str) {
        let message = c_string(&format!("{what} must satisfy {}", describe(rule)));
        if rule.each {
            let Type::List(item) = ty else { return };
            let index = self.variable("index", &Type::Int);
            self.open(format!(
                "for ({index} = 0; {index} < (dn_int) {value}.length; {index}++)"
            ));
            if let Some(test) = self.rule_test(&format!("{value}.items[{index}]"), item, rule) {
                self.line(format!("DN_CHECK({test}, {message});"));
            }
            self.close();
            return;
        }
        if let Some(test) = self.rule_test(value, ty, rule) {
            self.line(format!("DN_CHECK({test}, {message});"));
        }
    }

    /// A C expression that is true when `value` keeps the rule, or `None`
    /// when its arguments can't be worked out now.
    fn rule_test(&mut self, value: &str, ty: &Type, rule: &Rule) -> Option<String> {
        let constraint = &rule.constraint;
        let aggregate = self.aggregate.clone();
        let found = self
            .gen
            .model
            .resolve(&self.context, aggregate.as_deref(), &constraint.name);
        let numbers: Option<Vec<i64>> = (constraint.arguments.iter())
            .map(|argument| evaluate(self.gen.program, &self.gen.model, &self.context, argument))
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
                    ("Length", [exact]) => Some(format!("((dn_int) {value}.length == {exact}L)")),
                    ("Length", [low, high]) => Some(format!(
                        "((dn_int) {value}.length >= {low}L && (dn_int) {value}.length <= {high}L)"
                    )),
                    _ => None,
                }
            }
        }
    }
}

/// A constant argument as a literal, so the checker's notes aren't needed.
fn literal(number: i64, original: &dn_syntax::ast::Expr) -> dn_syntax::ast::Expr {
    dn_syntax::ast::Expr {
        kind: dn_syntax::ast::ExprKind::Int(number),
        span: original.span,
        id: original.id,
    }
}

/// `Range(1, 50)` or `each Alive`, as written, for messages.
fn describe(rule: &Rule) -> String {
    let name = rule.constraint.name.text();
    let arguments: Vec<String> = rule
        .constraint
        .arguments
        .iter()
        .map(describe_argument)
        .collect();
    let call = if arguments.is_empty() {
        name
    } else {
        format!("{name}({})", arguments.join(", "))
    };
    if rule.each {
        format!("each {call}")
    } else {
        call
    }
}

fn describe_argument(value: &dn_syntax::ast::Expr) -> String {
    use dn_syntax::ast::{BinaryOp, ExprKind, UnaryOp};
    match &value.kind {
        ExprKind::Int(number) => number.to_string(),
        ExprKind::Name(name) => name.clone(),
        ExprKind::Unary(UnaryOp::Negate, inner) => format!("-{}", describe_argument(inner)),
        ExprKind::Binary(op, left, right) => {
            let symbol = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
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
