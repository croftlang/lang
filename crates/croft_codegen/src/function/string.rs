//! The built-in methods of strings, using the runtime's string functions.

use croft_analysis::Type;
use croft_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// `text` is a `croft_string_view`, kept in a variable so the string it
    /// borrows from is read once, of a string of type `bounded`. A string
    /// made from it is no longer than it. When what it borrows stays put,
    /// `trim`, `slice` and each part of `split` borrow from it too, rather
    /// than being copied.
    pub fn string_method(
        &mut self,
        (text, bounded): (&str, &Type),
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let stable = self.is_stable(text);
        let owned = self.gen.owned_type(bounded);
        let part_type = Type::Borrowed(self.gen.bound(bounded) as u16);

        let borrowed = self.raw_variable("text", "croft_string_view");
        self.line(format!("{borrowed} = {text};"));
        let text = borrowed;

        let part = |writer: &mut Self, index: usize| match arguments.get(index) {
            Some(argument) => writer.view(&argument.value),
            None => "croft_string_view_from(\"\", 0)".to_string(),
        };

        let number = |writer: &mut Self, index: usize| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, &Type::Int),
            None => "0".to_string(),
        };

        match name {
            "contains" => {
                let part = part(self, 0);
                format!("(croft_string_find({text}, {part}, 0) >= 0)")
            }
            "starts_with" | "ends_with" => {
                let part = part(self, 0);
                format!("croft_string_{name}({text}, {part})")
            }
            "to_upper" | "to_lower" => {
                let temp = self.temp(&owned);
                self.line(format!("croft_string_{name}({text}, CROFT_OUT({temp}));"));
                temp
            }
            "trim" | "slice" => {
                let part = match name {
                    "trim" => format!("croft_string_trimmed({text})"),
                    _ => {
                        let start = number(self, 0);
                        let end = number(self, 1);
                        format!("croft_string_part({text}, {start}, {end})")
                    }
                };

                if stable {
                    let temp = self.temp(&part_type);
                    self.line(format!("{temp} = {part};"));
                    return temp;
                }

                let temp = self.temp(&owned);
                self.line(format!("croft_string_copy({part}, CROFT_OUT({temp}));"));
                temp
            }
            "to_int" => {
                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_string_to_int({text}, &{temp}.value);"
                ));

                temp
            }
            "to_bytes" => {
                // As many values as the string has characters, when it has a
                // bound of its own; a full string can be a longer literal.
                let held = match bounded {
                    Type::Text(_) | Type::Borrowed(_) => {
                        self.gen.bytes_type(self.gen.bound(bounded))
                    }
                    _ => Type::Bytes,
                };

                let temp = self.temp(&held);
                self.line(format!(
                    "croft_string_to_bytes({text}, CROFT_BYTES_OUT({temp}));"
                ));
                temp
            }
            "split" => {
                let separator = self.raw_variable("separator", "croft_string_view");
                let value = part(self, 0);
                self.line(format!("{separator} = {value};"));
                let item = if stable { part_type } else { owned };
                self.split(&text, &separator, &item)
            }
            _ => "0".to_string(),
        }
    }

    /// `line.split(",")`: the parts between each separator, each borrowed
    /// from `text` or copied, as `item` says.
    fn split(&mut self, text: &str, separator: &str, item: &Type) -> String {
        let parts = self.temp(&Type::list(item.clone()));
        let start = self.variable("start", &Type::Int);
        let found = self.variable("found", &Type::Int);

        self.line(format!("{parts}.length = 0;"));
        self.line(format!("{start} = 0;"));
        self.open("for (;;)");

        self.line(format!(
            "{found} = croft_string_find({text}, {separator}, {start});"
        ));

        let part =
            format!("croft_string_part({text}, {start}, {found} < 0 ? {text}.length : {found})");

        let slot = format!("{parts}.items[{parts}.length]");

        match item {
            Type::Borrowed(_) => self.line(format!("{slot} = {part};")),
            _ => self.line(format!("croft_string_copy({part}, CROFT_OUT({slot}));")),
        }

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
