//! The built-in methods of strings, using the runtime's string functions.

use croft_analysis::Type;
use croft_syntax::ast::Argument;

use super::Writer;
use crate::types::bounds::list_item;

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
            "code_at" => {
                let index = number(self, 0);
                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_string_code_at({text}, {index}, &{temp}.value);"
                ));

                temp
            }
            "to_decimal" => {
                let places = match ty {
                    Type::Option(item) => match **item {
                        Type::Decimal(places) => places,
                        _ => 0,
                    },
                    _ => 0,
                };

                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_string_to_decimal({text}, {places}, &{temp}.value);"
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
            "percent_encode" => {
                // At most three characters for each one.
                let most = self.gen.bound(bounded) * 3;
                let temp = self.temp(&self.gen.text_type(most));

                self.line(format!(
                    "croft_string_percent_encode({text}, CROFT_OUT({temp}));"
                ));

                temp
            }
            "percent_decode" => {
                let temp = self.temp(&Type::option(owned));

                self.line(format!(
                    "{temp}.some = croft_string_percent_decode({text}, CROFT_OUT({temp}.value));"
                ));

                temp
            }
            "split" => {
                let separator = self.raw_variable("separator", "croft_string_view");
                let value = part(self, 0);
                self.line(format!("{separator} = {value};"));
                let item = if stable { part_type } else { owned };
                // A string of n characters splits into at most n + 1 parts.
                let parts = self.gen.list_type(item, self.gen.bound(bounded) + 1);
                self.split(&text, &separator, &parts)
            }
            "lines" => {
                let item = if stable { part_type } else { owned };
                // Every line but an empty last one, which isn't kept, takes
                // at least one character.
                let lines = self.gen.list_type(item, self.gen.bound(bounded));
                self.lines(&text, &lines)
            }
            "index_of" => {
                let part = part(self, 0);
                let temp = self.temp(ty);
                self.line(format!(
                    "{temp}.value = croft_string_find({text}, {part}, 0);"
                ));
                self.line(format!("{temp}.some = {temp}.value >= 0;"));
                temp
            }
            "replace" => {
                let from = part(self, 0);

                let (to, to_type) = match arguments.get(1) {
                    Some(argument) => self.bounded_view(&argument.value),
                    None => ("croft_string_view_from(\"\", 0)".to_string(), Type::Text(1)),
                };

                // Each character changed, at most, for all of `to`.
                let most = self.gen.bound(bounded) * self.gen.bound(&to_type).max(1);
                let temp = self.temp(&self.gen.text_type(most));

                self.line(format!(
                    "croft_string_replace({text}, {from}, {to}, CROFT_OUT({temp}));"
                ));

                temp
            }
            "repeat" => {
                let count = number(self, 0);
                let most = self.most(arguments.first()).map_or(i64::MAX, |high| {
                    self.gen.bound(bounded).saturating_mul(high)
                });
                let temp = self.temp(&self.gen.text_type(most));

                self.line(format!(
                    "croft_string_repeat({text}, {count}, CROFT_OUT({temp}));"
                ));

                temp
            }
            "pad_left" | "pad_right" => {
                let width = number(self, 0);
                let fill = part(self, 1);
                let before = u8::from(name == "pad_left");
                let most = self
                    .most(arguments.first())
                    .map_or(i64::MAX, |high| high.max(self.gen.bound(bounded)));
                let temp = self.temp(&self.gen.text_type(most));

                self.line(format!(
                    "croft_string_pad({text}, {width}, {fill}, {before}, CROFT_OUT({temp}));"
                ));

                temp
            }
            _ => "0".to_string(),
        }
    }

    /// The most a whole-number argument can be, when its range is known.
    fn most(&self, argument: Option<&Argument>) -> Option<i64> {
        let range = self.range_of(&argument?.value)?;
        Some(range.high.max(0))
    }

    /// `text.lines()`: each line, without its `\n` or `\r\n`, borrowed from
    /// `text` or copied, as the item type of `ty` says. A last line that
    /// ends in `\n` has no empty line after it.
    fn lines(&mut self, text: &str, ty: &Type) -> String {
        let item = list_item(ty).cloned().unwrap_or(Type::String);
        let lines = self.temp(ty);
        let at = self.raw_variable("at", "croft_u16");
        let capacity = self.gen.capacity(ty);

        self.line(format!("{lines}.length = 0;"));
        self.line(format!("{at} = 0;"));

        self.open(format!(
            "while ({at} < {text}.length && {lines}.length < {capacity})"
        ));

        let line = format!("croft_string_line({text}, &{at})");
        let slot = format!("{lines}.items[{lines}.length]");

        match item {
            Type::Borrowed(_) => self.line(format!("{slot} = {line};")),
            _ => self.line(format!("croft_string_copy({line}, CROFT_OUT({slot}));")),
        }

        self.line(format!("{lines}.length++;"));
        self.close();

        lines
    }

    /// `line.split(",")`: the parts between each separator, each borrowed
    /// from `text` or copied, as the item type of `ty` says.
    fn split(&mut self, text: &str, separator: &str, ty: &Type) -> String {
        let item = list_item(ty).cloned().unwrap_or(Type::String);
        let parts = self.temp(ty);
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

        let capacity = self.gen.capacity(ty);

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
