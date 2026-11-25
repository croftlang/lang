//! Blocks and their statements: `let`, `set`, guards and loops.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind, Statement};

use super::{usage, Returns, Writer};

impl<'g, 'a> Writer<'g, 'a> {
    /// Writes a block's statements and gives back its last line's value.
    pub fn block(&mut self, statements: &[Statement], want: &Type) -> String {
        self.push_scope();
        let mut result = "0".to_string();
        for (index, statement) in statements.iter().enumerate() {
            let is_last = index + 1 == statements.len();
            match statement {
                Statement::Let(name, _, value) => {
                    let ty = self.gen.facts.kept_as(value.id).clone();
                    if !usage::statements_use(&statements[index + 1..], &name.name) {
                        // Never read: work it out for its effects, with no variable.
                        self.eval(value, None);
                        continue;
                    }
                    let c = self.value(value, &ty);
                    let variable = self.variable(&format!("v_{}", name.name), &ty);
                    self.line(format!("{variable} = {c};"));
                    self.bind(&name.name, variable, ty);
                }
                Statement::Set(name, value) => {
                    let (target, ty) = self.set_target(&name.name);
                    let c = self.value(value, &ty);
                    let own = self
                        .own
                        .as_ref()
                        .filter(|_| self.lookup(&name.name).is_none());
                    match own.map(|own| {
                        (
                            own.prefix.trim_end_matches('.').to_string(),
                            own.owner.clone(),
                        )
                    }) {
                        Some((prefix, owner)) => {
                            self.write_field(owner.as_ref(), &prefix, &name.name, &c)
                        }
                        None => self.line(format!("{target} = {c};")),
                    }
                }
                Statement::Expr(value) => match &value.kind {
                    ExprKind::If(condition, block) => self.guard(condition, block),
                    _ if is_last => result = self.value(value, want),
                    _ => {
                        self.eval(value, None);
                    }
                },
            }
        }
        self.pop_scope();
        result
    }

    /// What `set name` changes: an own field, or a service's `let` name.
    fn set_target(&self, name: &str) -> (String, Type) {
        if let Some((c, ty)) = self.lookup(name) {
            return (c, ty);
        }
        let own = self.own.as_ref();
        let ty = own
            .and_then(|own| own.fields.iter().find(|(f, _)| f == name))
            .map_or(Type::Unknown, |(_, ty)| ty.clone());
        (self.local_or_field(name), ty)
    }

    /// A guard: when the block gives back a value, the function returns it;
    /// when it only takes steps, the function carries on after it.
    pub fn guard(&mut self, condition: &Expr, block: &Expr) {
        let condition = self.value(condition, &Type::Bool);
        let ExprKind::Block(statements) = &block.kind else {
            return;
        };
        self.open(format!("if ({condition})"));
        if self.is_steps(statements) {
            self.block(statements, &Type::Unit);
        } else {
            let result = match &self.returns {
                Returns::Value(ty) => ty.clone(),
                _ => Type::Unit,
            };
            let value = self.block(statements, &result);
            self.give_back(value);
        }
        self.close();
    }

    fn is_steps(&self, statements: &[Statement]) -> bool {
        match statements.last() {
            Some(Statement::Expr(value)) => {
                matches!(
                    value.kind,
                    ExprKind::If(..) | ExprKind::Loop(..) | ExprKind::Break | ExprKind::Emit(..)
                ) || self.type_of(value, None) == Type::Unit
            }
            _ => true,
        }
    }

    /// Ends the function with a value already of its result type.
    pub fn give_back(&mut self, value: String) {
        if let Returns::Value(_) = self.returns {
            self.line(format!("*out = {value};"));
        }
        self.line("return;");
    }
}
