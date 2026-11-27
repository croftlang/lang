//! `Bytes` in C: a length and a fixed array of `unsigned char`. Like a string,
//! each is kept in the most values it can need, a `croft_bytes_N`, or a full
//! `croft_bytes` of the target's `bytes_length` when nothing bounds it.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr, ExprKind};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// Where `value` can be read from as `Bytes`, and its bounded type. A
    /// borrowed parameter is read where it is, never copied just to be
    /// looked at; anything else is worked out in the size it has.
    pub fn bytes_place(&mut self, value: &Expr) -> (String, Type) {
        if let ExprKind::Name(name) = &value.kind {
            if let Some((c, ty)) = self.lookup(name) {
                if self.views.contains(&c) {
                    return (c, ty);
                }
            }
        }

        self.eval(value, None)
    }

    /// `value` as a `croft_bytes_view`, as a `Bytes` parameter takes it.
    pub fn bytes_view(&mut self, value: &Expr) -> String {
        let (c, _) = self.bytes_place(value);
        self.bytes_view_of(&c)
    }

    /// A view of `c`: any `Bytes`, or a borrowed parameter, which is a view
    /// already.
    pub fn bytes_view_of(&self, c: &str) -> String {
        if self.views.contains(c) {
            return c.to_string();
        }

        format!("CROFT_BYTES_VIEW({c})")
    }

    /// `[1, 2, 3]` as `Bytes`, in a size of exactly that many values.
    pub fn bytes_literal(&mut self, items: &[Expr]) -> String {
        let values: Vec<String> = items
            .iter()
            .map(|value| self.value(value, &Type::Int))
            .collect();

        let ty = self.gen.bytes_type(values.len() as i64);
        let bytes = self.temp(&ty);
        let capacity = self.gen.bound(&ty);

        self.line(format!(
            "{bytes}.length = {};",
            values.len().min(capacity as usize)
        ));

        if values.len() as i64 > capacity {
            self.line(format!(
                "CROFT_CHECK(0, \"bytes hold at most {capacity}\");"
            ));
        }

        for (index, value) in values.iter().take(capacity as usize).enumerate() {
            self.line(format!("{bytes}.items[{index}] = (unsigned char) {value};"));
        }

        // `[]` still has room for one value. Setting it keeps gcc from
        // warning that a view of it reads values never written.
        if values.is_empty() {
            self.line(format!("{bytes}.items[0] = 0;"));
        }

        bytes
    }

    /// A method of `bytes`, a C value of type `bounded`. `add` gives back
    /// room for one more value, `add_u16` for two more, and `set` and
    /// `slice` no more than it had.
    pub fn bytes_method(
        &mut self,
        (bytes, bounded): (&str, &Type),
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let argument = |writer: &mut Self, index: usize| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, &Type::Int),
            None => "0".to_string(),
        };

        let view = format!("CROFT_BYTES_VIEW({bytes})");
        let held = self.gen.bound(bounded);

        match name {
            "get" => {
                let index = argument(self, 0);
                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_bytes_get({view}, {index}, &{temp}.value);"
                ));

                temp
            }
            "add" => {
                let byte = argument(self, 0);
                let temp = self.temp(&self.gen.bytes_type(held + 1));

                self.line(format!(
                    "croft_bytes_add({view}, {byte}, CROFT_BYTES_OUT({temp}));"
                ));

                temp
            }
            "get_u16" | "get_u16_be" => {
                let index = argument(self, 0);
                let big = u8::from(name == "get_u16_be");
                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_bytes_get_u16({view}, {index}, {big}, &{temp}.value);"
                ));

                temp
            }
            "add_u16" | "add_u16_be" => {
                let number = argument(self, 0);
                let big = u8::from(name == "add_u16_be");
                let temp = self.temp(&self.gen.bytes_type(held + 2));

                self.line(format!(
                    "croft_bytes_add_u16({view}, {number}, {big}, CROFT_BYTES_OUT({temp}));"
                ));

                temp
            }
            "set" => {
                let index = argument(self, 0);
                let byte = argument(self, 1);
                let temp = self.temp(&self.gen.bytes_type(held));

                self.line(format!(
                    "croft_bytes_set({view}, {index}, {byte}, CROFT_BYTES_OUT({temp}));"
                ));

                temp
            }
            "slice" => {
                let start = argument(self, 0);
                let end = argument(self, 1);
                let temp = self.temp(&self.gen.bytes_type(held));

                self.line(format!(
                    "croft_bytes_slice({view}, {start}, {end}, CROFT_BYTES_OUT({temp}));"
                ));

                temp
            }
            "to_text" => {
                let temp = self.temp(&self.gen.text_type(held));
                self.line(format!("croft_bytes_to_text({view}, CROFT_OUT({temp}));"));
                temp
            }
            _ => "0".to_string(),
        }
    }
}
