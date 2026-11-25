//! Failing: `?` passes a failure up, and `or` fails with a chosen error.

use croft_analysis::Type;
use croft_syntax::ast::Expr;

use super::{Returns, Writer};

impl<'g, 'a> Writer<'g, 'a> {
    /// Ends the function with an error. A command also undoes its events.
    pub fn fail(&mut self, error: &str) {
        match &self.returns {
            Returns::Value(Type::Result(..)) => {
                self.line("out->ok = 0;");
                self.line(format!("out->error = {error};"));
            }
            Returns::Command => {
                self.line("croft_events_rollback(mark);");
                self.line("out->ok = 0;");
                self.line(format!("out->error = {error};"));
            }
            _ => {}
        }

        self.line("return;");
    }

    /// `value?`
    pub fn try_value(&mut self, inner: &Expr) -> String {
        let (result, _) = self.eval(inner, None);
        self.open(format!("if (!{result}.ok)"));
        self.fail(&format!("{result}.error"));
        self.close();
        format!("{result}.value")
    }

    /// `value or Error` on a `Result`, an `Option` or a `Bool`.
    pub fn or_else(&mut self, inner: &Expr, error: &Expr, _ty: &Type) -> String {
        let inner_type = self.type_of(inner, None);
        let (value, _) = self.eval(inner, None);

        let (failed, success) = match inner_type {
            Type::Result(..) => (format!("!{value}.ok"), format!("{value}.value")),
            Type::Option(_) => (format!("!{value}.some"), format!("{value}.value")),
            _ => (format!("!{value}"), "0".to_string()),
        };

        self.open(format!("if ({failed})"));

        let error = self.value(error, &Type::Errors(Vec::new()));
        self.fail(&error);
        self.close();

        success
    }
}
