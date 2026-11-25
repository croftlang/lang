//! Enums: a number when no variant carries data, otherwise a tag and a union.

use croft_analysis::types::Resolved;

use super::define::struct_definition;
use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    pub(super) fn enum_definition(&self, named: &Resolved) -> String {
        let name = c_name(named);
        let variants = self.model.variants(named);
        let tags: String = (variants.iter().enumerate())
            .map(|(index, variant)| format!("#define {name}_{} {index}\n", variant.name))
            .collect();
        if self.is_plain_enum(named) {
            return format!("typedef croft_u8 {name};\n{tags}\n");
        }
        let mut cases = String::new();
        for variant in variants.iter().filter(|v| !v.fields.is_empty()) {
            let fields: String = variant
                .fields
                .iter()
                .map(|(f, ty)| format!("            {} {f};\n", self.c_type(ty)))
                .collect();
            cases.push_str(&format!(
                "        struct {{\n{fields}        }} {};\n",
                variant.name
            ));
        }
        let body =
            format!("    croft_u8 tag;\n    union {{\n{cases}        croft_u8 none;\n    }} as;\n");
        format!("{tags}{}", struct_definition(&name, &body))
    }
}
