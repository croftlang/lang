//! Lists, maps, sets, options and results: one C struct for each item type used.

use croft_analysis::Type;

use super::define::struct_definition;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The C struct for a list, map, set, option or result type, or nothing for
    /// any other type.
    pub fn generic_definition(&self, ty: &Type) -> String {
        let name = self.c_type(ty);

        let fields = match ty {
            Type::List(item) | Type::SizedList(item, _) => {
                let capacity = self.capacity(ty);

                format!(
                    "    croft_u16 length;\n    {} items[{capacity}];\n",
                    self.c_type(item)
                )
            }
            Type::Map(key, value) => {
                let capacity = self.sizes.map_capacity(ty);

                format!(
                    "    croft_u16 length;\n    {} keys[{capacity}];\n    {} values[{capacity}];\n",
                    self.c_type(key),
                    self.c_type(value)
                )
            }
            Type::Set(item) => {
                let capacity = self.sizes.set_capacity(ty);

                format!(
                    "    croft_u16 length;\n    {} items[{capacity}];\n",
                    self.c_type(item)
                )
            }
            Type::Option(item) => {
                format!("    croft_bool some;\n    {} value;\n", self.c_type(item))
            }
            Type::Result(value, error) => {
                format!(
                    "    croft_bool ok;\n    {} value;\n    {} error;\n",
                    self.c_type(value),
                    self.c_type(error)
                )
            }
            _ => return String::new(),
        };

        struct_definition(&name, &fields)
    }
}
