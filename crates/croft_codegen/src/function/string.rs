//! The built-in methods of strings, using the runtime's string functions.

use croft_analysis::Type;
use croft_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn string_method(
        &mut self,
        text: &str,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let argument = |writer: &mut Self, index: usize, want: &Type| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };
        match name {
            "contains" => {
                let part = argument(self, 0, &Type::String);
                format!("(croft_string_find(&{text}, &{part}, 0) >= 0)")
            }
            "starts_with" | "ends_with" => {
                let part = argument(self, 0, &Type::String);
                format!("croft_string_{name}(&{text}, &{part})")
            }
            "to_upper" | "to_lower" | "trim" => {
                let temp = self.temp(ty);
                self.line(format!("croft_string_{name}(&{text}, &{temp});"));
                temp
            }
            "slice" => {
                let start = argument(self, 0, &Type::Int);
                let end = argument(self, 1, &Type::Int);
                let temp = self.temp(ty);
                self.line(format!(
                    "croft_string_slice(&{text}, {start}, {end}, &{temp});"
                ));
                temp
            }
            "to_int" => {
                let temp = self.temp(ty);
                self.line(format!(
                    "{temp}.some = croft_string_to_int(&{text}, &{temp}.value);"
                ));
                temp
            }
            "to_bytes" => {
                let temp = self.temp(ty);
                self.line(format!("croft_string_to_bytes(&{text}, &{temp});"));
                temp
            }
            "split" => {
                let separator = argument(self, 0, &Type::String);
                self.split(text, &separator, ty)
            }
            _ => "0".to_string(),
        }
    }

    /// `line.split(",")`: the parts between each separator.
    fn split(&mut self, text: &str, separator: &str, ty: &Type) -> String {
        let parts = self.temp(ty);
        let start = self.variable("start", &Type::Int);
        let found = self.variable("found", &Type::Int);
        self.line(format!("{parts}.length = 0;"));
        self.line(format!("{start} = 0;"));
        self.open("for (;;)");
        self.line(format!(
            "{found} = croft_string_find(&{text}, &{separator}, {start});"
        ));
        self.line(format!("croft_string_slice(&{text}, {start}, {found} < 0 ? {text}.length : {found}, &{parts}.items[{parts}.length]);"));
        self.line(format!("{parts}.length++;"));
        let capacity = self.gen.sizes.list_capacity(&Type::String);
        self.open(format!(
            "if ({found} < 0 || {separator}.length == 0 || {parts}.length >= {capacity})"
        ));
        self.line("break;");
        self.close();
        self.line(format!("{start} = {found} + {separator}.length;"));
        self.close();
        parts
    }
}
