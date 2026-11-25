//! Strings borrowed as a `croft_string_view`: a pointer to the characters
//! and a length, so a literal or a string parameter is read where it already
//! is, never copied into a string of its own just to be looked at.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind};

use super::Writer;
use crate::names::c_string;

impl<'g, 'a> Writer<'g, 'a> {
    /// A string expression as a `croft_string_view`, copying nothing that
    /// already has a place: a literal stays in the program's constant text.
    pub fn view(&mut self, value: &Expr) -> String {
        match &value.kind {
            ExprKind::String(text) => {
                format!(
                    "croft_string_view_from({}, {})",
                    c_string(text),
                    text.chars().count()
                )
            }
            ExprKind::Name(name) => match self.lookup(name) {
                Some((c, _)) if self.views.contains(&c) => c,
                _ => {
                    let c = self.value(value, &Type::String);

                    self.view_of(&c)
                }
            },
            _ => {
                let c = self.value(value, &Type::String);

                self.view_of(&c)
            }
        }
    }

    /// A view of `c`, a `croft_string` or a view already.
    pub fn view_of(&self, c: &str) -> String {
        if self.views.contains(c) {
            return c.to_string();
        }

        match c
            .strip_prefix("(*")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            Some(pointer) if !pointer.contains(['(', ')', '.', ' ']) => {
                format!("croft_string_view_of({pointer})")
            }
            _ => format!("croft_string_view_of(&{c})"),
        }
    }

    /// `c` as a `croft_string` of its own: a borrowed parameter is copied
    /// only when it is kept or passed on as a whole value.
    pub fn owned(&mut self, c: String) -> String {
        if !self.views.contains(&c) {
            return c;
        }

        let temp = self.temp(&Type::String);
        self.line(format!("croft_string_from_view({c}, &{temp});"));

        temp
    }

    /// Works out `value` straight into `target`, such as `(*out)`, and gives
    /// back `target`. A string literal is copied in once, and each arm of a
    /// `match` writes into it, so neither needs a temporary of its own.
    pub fn value_into(&mut self, target: &str, value: &Expr, ty: &Type) -> String {
        match &value.kind {
            ExprKind::String(text) if *ty == Type::String => {
                self.line(format!("croft_string_set(&{target}, {});", c_string(text)));
            }
            ExprKind::Match(subject, arms) if *ty != Type::Unit => {
                self.match_into(subject, arms, ty, target);
            }
            _ => {
                let c = self.value(value, ty);
                self.line(format!("{target} = {c};"));
            }
        }

        target.to_string()
    }
}
