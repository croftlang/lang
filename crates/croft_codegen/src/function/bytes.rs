//! `Bytes` in C: a length and a fixed array of `unsigned char`. Like a string,
//! each is kept in the most values it can need, a `croft_bytes_N`, or a full
//! `croft_bytes` as large as the longest `Length` on any `Bytes` allows.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
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

        bytes
    }

    /// A method of `bytes`, a C value of type `bounded`. `add` gives back
    /// room for one more value, and `slice` no more than it had.
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
