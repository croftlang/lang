//! Built-in functions: the numbers of `Std.Numbers`, `from_code` and
//! `from_hex` of `Std.Text`, `Some`, `Ok`, `Err`, `Range` and `Length`.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, ExprKind};

use super::range::WHOLE_LOW;
use super::{Into, Writer};
use crate::types::bounds::{is_string, unbounded};

impl<'g, 'a> Writer<'g, 'a> {
    /// A call to a built-in function such as `max` or `Some`.
    pub fn builtin(
        &mut self,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
        into: Into<'_>,
    ) -> String {
        let value = |writer: &mut Self, index: usize, want: &Type| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };

        match name {
            "max" | "min" | "abs" => {
                let first = value(self, 0, ty);

                // Only the lowest whole number has no answer, needing 33 bits.
                if name == "abs" {
                    return match arguments.first().and_then(|a| self.range_of(&a.value)) {
                        Some(range) if range.low > WHOLE_LOW => format!("CROFT_ABS({first})"),
                        _ => format!("croft_int_abs({first})"),
                    };
                }

                let second = value(self, 1, ty);

                format!("CROFT_{}({first}, {second})", name.to_uppercase())
            }
            "clamp" => {
                let number = value(self, 0, ty);
                let low = value(self, 1, ty);
                let high = value(self, 2, ty);
                let kind = if *ty == Type::Float { "float" } else { "int" };

                format!("croft_{kind}_clamp({number}, {low}, {high})")
            }
            "sign" => {
                let given = arguments
                    .first()
                    .map_or(Type::Int, |a| self.type_of(&a.value, None));

                format!("CROFT_SIGN({})", value(self, 0, &given))
            }
            "pow" => {
                let base = value(self, 0, &Type::Int);
                let exponent = value(self, 1, &Type::Int);

                format!("croft_int_pow({base}, {exponent})")
            }
            "checked_add" | "checked_subtract" | "checked_multiply" => {
                let a = value(self, 0, &Type::Int);
                let b = value(self, 1, &Type::Int);
                let temp = self.place_for(ty, into);

                self.line(format!(
                    "{temp}.some = croft_int_{name}({a}, {b}, &{temp}.value);"
                ));

                temp
            }
            "from_code" => {
                let code = value(self, 0, &Type::Int);
                let temp = self.temp(&self.gen.text_type(1));

                self.line(format!(
                    "croft_string_from_code({code}, CROFT_OUT({temp}));"
                ));

                temp
            }
            "from_hex" => {
                let text = match arguments.first() {
                    Some(argument) => self.view(&argument.value),
                    None => "croft_string_view_from(\"\", 0)".to_string(),
                };

                let temp = self.place_for(ty, into);

                self.line(format!(
                    "{temp}.some = croft_string_from_hex({text}, &{temp}.value);"
                ));

                temp
            }
            "Some" | "Ok" | "Err" => {
                let (flag, field, on, inner) = match ty {
                    Type::Option(item) => ("some", "value", "1", (**item).clone()),
                    Type::Result(item, _) if name == "Ok" => ("ok", "value", "1", (**item).clone()),
                    Type::Result(_, error) => ("ok", "error", "0", (**error).clone()),
                    _ => return "0".to_string(),
                };

                let Some(argument) = arguments.first() else {
                    return "0".to_string();
                };

                // With no strings to bound, the value is made straight in place.
                if unbounded(&inner) == self.gen.narrowest(&unbounded(&inner)) {
                    let made = ty.clone();
                    let temp = self.place_for(&made, into);
                    self.line(format!("{temp}.{flag} = {on};"));
                    self.value_into(&format!("{temp}.{field}"), &argument.value, &inner);
                    return temp;
                }

                // `Some(x)` and `Ok(x)` keep the bounds of `x`. `Err(e)` holds
                // no value, so its value takes the least room.
                let (c, given) = match &argument.value.kind {
                    ExprKind::String(text) if is_string(&inner) => {
                        (None, self.gen.text_type(text.chars().count() as i64))
                    }
                    _ => {
                        let (c, bounded) = self.eval(&argument.value, Some(&inner));
                        let given = self.gen.keep_bound(&bounded, &inner);
                        (Some(self.convert(c, &bounded, &given)), given)
                    }
                };

                let made = match ty {
                    Type::Option(_) => Type::option(given.clone()),
                    Type::Result(_, error) if name == "Ok" => {
                        Type::result(given.clone(), (**error).clone())
                    }
                    Type::Result(value, _) => {
                        Type::result(self.gen.narrowest(value), given.clone())
                    }
                    _ => ty.clone(),
                };

                let temp = self.place_for(&made, into);
                self.line(format!("{temp}.{flag} = {on};"));
                let place = format!("{temp}.{field}");

                match c {
                    Some(c) => self.copy_into(&place, &c, &given, &given),
                    None => self.assign(&place, &argument.value, &given),
                }

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

                format!(
                    "((croft_int) {items}.length >= {low} && (croft_int) {items}.length <= {high})"
                )
            }
            _ => "0".to_string(),
        }
    }
}
