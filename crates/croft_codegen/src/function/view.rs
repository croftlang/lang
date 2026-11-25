//! Strings borrowed as a `croft_string_view`: a pointer to the characters
//! and a length, so a literal or a string parameter is read where it already
//! is, never copied into a string of its own just to be looked at.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind};

use super::Writer;
use crate::names::c_string;
use crate::types::bounds::is_string;

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
        if self.views.contains(c) {
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
            _ => self.assign(target, value, ty),
        }

        target.to_string()
    }
}
