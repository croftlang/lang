//! Blocks and their statements: `let`, `set`, guards and loops.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind, Statement};

use super::{usage, Returns, Writer};

impl<'g, 'a> Writer<'g, 'a> {
    /// Writes a block's statements and gives back its last line's value.
    pub fn block(&mut self, statements: &[Statement], want: &Type) -> String {
        self.block_to(statements, want, None)
    }

    /// Writes a block's statements, its last line's value straight into
    /// `target`, a place of type `want`, and gives back `target`.
    pub fn block_into(&mut self, statements: &[Statement], target: &str, want: &Type) -> String {
        self.block_to(statements, want, Some(target))
    }

    fn block_to(&mut self, statements: &[Statement], want: &Type, into: Option<&str>) -> String {
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

                    let handed = self.temps_mark();
                    // A name `set` later needs room for whatever it is set to.
                    let set_later = usage::statements_set(&statements[index + 1..], &name.name);

                    let (c, ty) = if set_later {
                        let c = self.value(value, &ty);

                        // What it holds changes, so nothing may borrow from it.
                        self.stable.remove(&c);

                        (c, ty)
                    } else {
                        let (c, bounded) = self.eval(value, Some(&ty));
                        let kept = self.gen.keep_bound(&bounded, &ty);
                        (self.convert(c, &bounded, &kept), kept)
                    };

                    // A temporary made just now for this value becomes the
                    // name itself, rather than being copied into another.
                    let fresh = self.handed_since(handed, &c);

                    if fresh {
                        self.bind(&name.name, c, ty);
                        continue;
                    }

                    let variable = self.variable(&format!("v_{}", name.name), &ty);
                    self.line(format!("{variable} = {c};"));
                    self.bind(&name.name, variable, ty);
                }
                Statement::Set(name, value) => {
                    let (target, ty) = self.set_target(&name.name);

                    let own = self
                        .own
                        .as_ref()
                        .filter(|_| self.lookup(&name.name).is_none())
                        .map(|own| {
                            (
                                own.prefix.trim_end_matches('.').to_string(),
                                own.owner.clone(),
                            )
                        });

                    let kept_in_own_size = (own.as_ref())
                        .and_then(|(_, owner)| owner.as_ref())
                        .and_then(|owner| self.gen.text_capacity(owner, &name.name));

                    // Any string can be written into a field of its own size.
                    let c = match kept_in_own_size {
                        Some(_) => self.view(value),
                        None => self.value(value, &ty),
                    };

                    match own {
                        Some((prefix, owner)) => {
                            self.write_field(owner.as_ref(), &prefix, &name.name, &c)
                        }
                        None => self.line(format!("{target} = {c};")),
                    }
                }
                Statement::Expr(value) => match &value.kind {
                    ExprKind::If(condition, block) => self.guard(condition, block),
                    _ if is_last => {
                        result = match into {
                            Some(target) => self.value_into(target, value, want),
                            None => self.bounded_value(value, want),
                        }
                    }
                    _ => {
                        self.eval(value, None);
                    }
                },
            }
        }

        self.pop_scope();

        // A block that ends in a step, not a value, gives back `Unit`: as a
        // successful result, when one is wanted.
        if let Some(target) = into {
            if result != target {
                self.store(target, result, &Type::Unit, want);
            }

            return target.to_string();
        }

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
        } else if let Returns::Value(result) = &self.returns {
            // The value is written straight into the result, and once the
            // function has returned, its temporaries can be used again.
            let result = result.clone();
            let mark = self.temps_mark();
            self.block_into(statements, "(*out)", &result);
            self.line("return;");
            self.release_since(mark);
        } else {
            let value = self.block(statements, &Type::Unit);
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

    /// The value of `value` for a place of type `want`, keeping the bounds
    /// of its strings: whoever takes it copies it to the size it needs.
    pub fn bounded_value(&mut self, value: &Expr, want: &Type) -> String {
        let (c, ty) = self.eval(value, Some(want));
        let kept = self.gen.keep_bound(&ty, want);

        self.convert(c, &ty, &kept)
    }

    /// Ends the function with a value of its result type, or one that
    /// differs only in the bounds of its strings.
    pub fn give_back(&mut self, value: String) {
        if let Returns::Value(result) = &self.returns {
            let result = result.clone();
            let from = self.bounded_type(&value, &result);
            self.copy_into("(*out)", &value, &from, &result);
        }

        self.line("return;");
    }
}
