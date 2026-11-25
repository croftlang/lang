//! Built-in functions: `max`, `min`, `abs`, `Some`, `Ok`, `Err`, `Range` and `Length`.

use b_analysis::Type;
use b_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn builtin(&mut self, name: &str, arguments: &[Argument], ty: &Type) -> String {
        let value = |writer: &mut Self, index: usize, want: &Type| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };
        match name {
            "max" | "min" | "abs" => {
                let first = value(self, 0, ty);
                if name == "abs" {
                    return format!("B_ABS({first})");
                }
                let second = value(self, 1, ty);
                format!("B_{}({first}, {second})", name.to_uppercase())
            }
            "Some" | "Ok" | "Err" => {
                let (flag, field, on, inner) = match ty {
                    Type::Option(item) => ("some", "value", "1", (**item).clone()),
                    Type::Result(item, _) if name == "Ok" => ("ok", "value", "1", (**item).clone()),
                    Type::Result(_, error) => ("ok", "error", "0", (**error).clone()),
                    _ => return "0".to_string(),
                };
                let inner = value(self, 0, &inner);
                let temp = self.temp(ty);
                self.line(format!("{temp}.{flag} = {on};"));
                self.line(format!("{temp}.{field} = {inner};"));
                temp
            }
            "Range" => {
                let number = value(self, 0, &Type::Int);
                let low = value(self, 1, &Type::Int);
                let high = value(self, 2, &Type::Int);
                format!("({number} >= {low} && {number} <= {high})")
            }
            "Length" => {
                let items_type = arguments
                    .first()
                    .map_or(Type::Unknown, |a| self.type_of(&a.value, None));
                let items = value(self, 0, &items_type);
                let low = value(self, 1, &Type::Int);
                let high = if arguments.len() > 2 {
                    value(self, 2, &Type::Int)
                } else {
                    low.clone()
                };
                format!("((b_int) {items}.length >= {low} && (b_int) {items}.length <= {high})")
            }
            _ => "0".to_string(),
        }
    }
}
