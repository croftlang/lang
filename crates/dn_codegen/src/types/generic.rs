//! Lists, options and results: one C struct for each item type used.

use dn_analysis::Type;

use super::define::struct_definition;
use crate::Generator;

impl<'a> Generator<'a> {
    pub fn generic_definition(&self, ty: &Type) -> String {
        let name = self.c_type(ty);
        let fields = match ty {
            Type::List(item) => {
                let capacity = self.sizes.list_capacity(item);
                format!(
                    "    dn_u16 length;\n    {} items[{capacity}];\n",
                    self.c_type(item)
                )
            }
            Type::Option(item) => format!("    dn_bool some;\n    {} value;\n", self.c_type(item)),
            Type::Result(value, error) => {
                format!(
                    "    dn_bool ok;\n    {} value;\n    {} error;\n",
                    self.c_type(value),
                    self.c_type(error)
                )
            }
            _ => return String::new(),
        };
        struct_definition(&name, &fields)
    }
}
