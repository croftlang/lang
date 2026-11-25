//! `Bytes` in C: a length and a fixed array of `unsigned char`, as large as the
//! longest `Length` constraint on any `Bytes` allows.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn bytes_literal(&mut self, items: &[Expr]) -> String {
        let values: Vec<String> = items
            .iter()
            .map(|value| self.value(value, &Type::Int))
            .collect();

        let bytes = self.temp(&Type::Bytes);
        let capacity = self.gen.sizes.bytes_capacity;

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

    pub fn bytes_method(
        &mut self,
        bytes: &str,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let argument = |writer: &mut Self, index: usize| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, &Type::Int),
            None => "0".to_string(),
        };

        match name {
            "get" => {
                let index = argument(self, 0);
                let temp = self.temp(ty);

                self.line(format!(
                    "{temp}.some = croft_bytes_get(&{bytes}, {index}, &{temp}.value);"
                ));

                temp
            }
            "add" => {
                let byte = argument(self, 0);
                let temp = self.temp(ty);
                self.line(format!("croft_bytes_add(&{bytes}, {byte}, &{temp});"));
                temp
            }
            "slice" => {
                let start = argument(self, 0);
                let end = argument(self, 1);
                let temp = self.temp(ty);

                self.line(format!(
                    "croft_bytes_slice(&{bytes}, {start}, {end}, &{temp});"
                ));

                temp
            }
            "to_text" => {
                let temp = self.temp(ty);
                self.line(format!("croft_bytes_to_text(&{bytes}, CROFT_OUT({temp}));"));
                temp
            }
            _ => "0".to_string(),
        }
    }
}
