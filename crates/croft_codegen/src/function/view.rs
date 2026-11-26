//! Strings borrowed as a `croft_string_view`: a pointer to the characters
//! and a length, so a literal or a string parameter is read where it already
//! is, never copied into a string of its own just to be looked at.

use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{BinaryOp, Expr, ExprKind};

use super::Writer;
use crate::names::{c_name, c_string};
use crate::types::bounds::{is_bytes, is_string};

impl<'g, 'a> Writer<'g, 'a> {
    /// A string expression as a `croft_string_view`, copying nothing that
    /// already has a place: a literal stays in the program's constant text.
    pub fn view(&mut self, value: &Expr) -> String {
        self.bounded_view(value).0
    }

    /// A string expression as a view, and the bounded string type of what
    /// it borrows: a literal is as long as it is.
    pub fn bounded_view(&mut self, value: &Expr) -> (String, Type) {
        match &value.kind {
            ExprKind::String(text) => {
                let length = text.chars().count();

                (
                    format!("croft_string_view_from({}, {length})", c_string(text)),
                    self.gen.text_type(length as i64),
                )
            }
            ExprKind::Name(name) => match self.lookup(name) {
                Some((c, ty)) if self.views.contains(&c) => (c, ty),
                _ => {
                    let (c, ty) = self.eval(value, Some(&Type::String));

                    (self.view_of(&c), ty)
                }
            },
            _ => {
                let (c, ty) = self.eval(value, Some(&Type::String));

                (self.view_of(&c), ty)
            }
        }
    }

    /// A view of `c`: any string, or a view already.
    pub fn view_of(&self, c: &str) -> String {
        let is_view = self.views.contains(c)
            || matches!(self.bounded.get(c), Some(Type::Borrowed(_)))
            || c.starts_with("croft_string_view_from(");

        if is_view {
            return c.to_string();
        }

        format!("CROFT_VIEW({c})")
    }

    /// `c` as a string of its own, of type `ty`: a borrowed parameter is
    /// copied only when it is kept or passed on as a whole value.
    pub fn owned(&mut self, c: String, ty: &Type) -> String {
        if !self.views.contains(&c) {
            return c;
        }

        let temp = self.temp(ty);
        self.line(format!("croft_string_copy({c}, CROFT_OUT({temp}));"));

        temp
    }

    /// Works out `value` straight into `target`, such as `(*out)`, and gives
    /// back `target`. A string literal is copied in once, and each arm of a
    /// `match` writes into it, so neither needs a temporary of its own.
    pub fn value_into(&mut self, target: &str, value: &Expr, ty: &Type) -> String {
        match &value.kind {
            ExprKind::String(_) if is_string(ty) => self.assign(target, value, ty),
            ExprKind::Match(subject, arms) if *ty != Type::Unit => {
                self.match_into(subject, arms, ty, target);
            }
            ExprKind::Block(statements) => {
                self.block_into(statements, target, ty);
            }
            // A call or command writes its result straight into the target
            // when that has its C type.
            ExprKind::Call(callee, arguments) => {
                let made = self.type_of(value, Some(ty));
                let c = self.call(value, callee, arguments, &made, Some((target, ty)));
                let from = self.bounded_type(&c, &made);
                self.store(target, c, &from, ty);
            }
            // Joined text goes straight into a target with room for all of it.
            ExprKind::Binary(BinaryOp::Add, left, right) if is_string(ty) => {
                let (left, left_type) = self.bounded_view(left);
                let (right, right_type) = self.bounded_view(right);
                let length = self.gen.bound(&left_type) + self.gen.bound(&right_type);
                let joined = self.gen.text_type(length);

                if self.gen.bound(&joined) <= self.gen.bound(ty) {
                    self.line(format!(
                        "croft_string_join({left}, {right}, CROFT_OUT({target}));"
                    ));

                    self.note_result(target, &joined);
                } else {
                    let temp = self.temp(&joined);

                    self.line(format!(
                        "croft_string_join({left}, {right}, CROFT_OUT({temp}));"
                    ));

                    self.store(target, temp, &joined, ty);
                }
            }
            // Joined bytes go straight into a target with room for all of them.
            ExprKind::Binary(BinaryOp::Add, left, right) if is_bytes(ty) => {
                let (left, right, joined) = self.bytes_parts(left, right);

                if self.gen.bound(&joined) <= self.gen.bound(ty) {
                    self.line(format!(
                        "croft_bytes_join({left}, {right}, CROFT_BYTES_OUT({target}));"
                    ));

                    self.note_result(target, &joined);
                } else {
                    let temp = self.temp(&joined);

                    self.line(format!(
                        "croft_bytes_join({left}, {right}, CROFT_BYTES_OUT({temp}));"
                    ));

                    self.store(target, temp, &joined, ty);
                }
            }
            // A variant without fields is just its tag.
            ExprKind::Name(_) | ExprKind::Field(..) => match self.gen.facts.meaning(value.id) {
                Some(Meaning::Variant(owner, variant, None))
                    if !self.gen.is_plain_enum(owner)
                        && !self.gen.model.is_error(&Type::Named(owner.clone()))
                        && self.gen.c_type(&Type::Named(owner.clone())) == self.gen.c_type(ty) =>
                {
                    self.line(format!("{target}.tag = {}_{variant};", c_name(owner)));
                    self.note_result(target, ty);
                }
                _ => self.assign(target, value, ty),
            },
            // `Err(e)?` always leaves the function, so it never gives a value
            // to write.
            ExprKind::Try(_) if *self.gen.facts.type_of(value.id) == Type::Unknown => {
                self.eval(value, None);
            }
            ExprKind::Issue(issue) => {
                let made = self.type_of(value, Some(ty));
                let c = self.issue(value, issue, &made, Some((target, ty)));
                let from = self.bounded_type(&c, &made);
                self.store(target, c, &from, ty);
            }
            _ => self.assign(target, value, ty),
        }

        target.to_string()
    }
}
