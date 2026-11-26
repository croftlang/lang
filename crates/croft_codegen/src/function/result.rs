//! The built-in methods of results.

use croft_analysis::Type;
use croft_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// A method of `result`, a C value of type `Result<value, error>`.
    pub(super) fn result_method(
        &mut self,
        result: &str,
        (value, error): (&Type, &Type),
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        if name == "unwrap_or" {
            let (present, held) = (format!("{result}.ok"), format!("{result}.value"));
            return self.unwrap_or((&present, &held), value, arguments, ty);
        }

        let Type::Result(mapped_value, mapped_error) = ty else {
            return "0".to_string();
        };

        // What the lambda doesn't change keeps its bounds.
        let ty = match name {
            "map" => Type::result((**mapped_value).clone(), error.clone()),
            _ => Type::result(value.clone(), (**mapped_error).clone()),
        };

        let temp = self.temp(&ty);

        self.line(format!("{temp}.ok = {result}.ok;"));
        self.open(format!("if ({result}.ok)"));

        match name {
            "map" => {
                let held = format!("{result}.value");
                let mapped = self.lambda(arguments, held, value, mapped_value);
                self.line(format!("{temp}.value = {mapped};"));
            }
            _ => self.line(format!("{temp}.value = {result}.value;")),
        }

        self.close();
        self.open("else");

        match name {
            "map" => self.line(format!("{temp}.error = {result}.error;")),
            _ => {
                let held = format!("{result}.error");
                let mapped = self.lambda(arguments, held, error, mapped_error);
                self.line(format!("{temp}.error = {mapped};"));
            }
        }

        self.close();

        temp
    }
}
