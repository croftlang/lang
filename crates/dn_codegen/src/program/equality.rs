//! Equality for structs, lists, options and errors, written as functions on demand.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;

use crate::Generator;

impl<'a> Generator<'a> {
    /// The name of the function that compares two values of `ty`, generated later.
    pub fn equality_for(&self, ty: &Type) -> String {
        let ty = self.canonical(ty);
        let name = format!("dn_equal_{}", self.c_type(&ty).trim_start_matches("dn_"));
        let mut equalities = self.equalities.borrow_mut();
        if equalities.1.insert(ty.clone()) {
            equalities.0.push(ty);
        }
        name
    }

    /// Every equality function asked for, including those they need in turn.
    pub fn equality_functions(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();
        let mut index = 0;
        loop {
            let next = self.equalities.borrow().0.get(index).cloned();
            let Some(ty) = next else { break };
            index += 1;
            let name = self.equality_for(&ty);
            let c_type = self.c_type(&ty);
            let signature = format!("dn_bool {name}(const {c_type} *a, const {c_type} *b)");
            prototypes.push(format!("{signature};"));
            let body = self.equality_body(&ty);
            out.push_str(&format!("{signature}\n{{\n{body}}}\n\n"));
        }
        out
    }

    fn equality_body(&self, ty: &Type) -> String {
        match ty {
            Type::List(item) => {
                let same = self.compare("a->items[index]", "b->items[index]", item);
                format!(
                    "    dn_u16 index;\n\n    if (a->length != b->length) {{\n        return 0;\n    }}\n    \
                     for (index = 0; index < a->length; index++) {{\n        if (!{same}) {{\n            return 0;\n        }}\n    }}\n    return 1;\n"
                )
            }
            // The same keys with the same values, in any order.
            Type::Map(key, value) => {
                let same_key = self.compare("a->keys[index]", "b->keys[other]", key);
                let same_value = self.compare("a->values[index]", "b->values[other]", value);
                format!(
                    "    dn_u16 index;\n    dn_u16 other;\n\n    if (a->length != b->length) {{\n        return 0;\n    }}\n    \
                     for (index = 0; index < a->length; index++) {{\n        \
                     for (other = 0; other < b->length && !{same_key}; other++) {{\n        }}\n        \
                     if (other == b->length || !{same_value}) {{\n            return 0;\n        }}\n    }}\n    return 1;\n"
                )
            }
            Type::Option(item) => {
                let same = self.compare("a->value", "b->value", item);
                format!("    return a->some == b->some && (!a->some || {same});\n")
            }
            Type::Errors(_) => "    return a->code == b->code;\n".to_string(),
            Type::Named(named) if named.kind == SymbolKind::Enum => {
                let mut tests =
                    String::from("    if (a->tag != b->tag) {\n        return 0;\n    }\n");
                for variant in self.model.variants(named) {
                    for (field, field_type) in &variant.fields {
                        let same = self.compare(
                            &format!("a->as.{}.{field}", variant.name),
                            &format!("b->as.{}.{field}", variant.name),
                            field_type,
                        );
                        let tag = format!("{}_{}", crate::names::c_name(named), variant.name);
                        tests.push_str(&format!(
                            "    if (a->tag == {tag} && !{same}) {{\n        return 0;\n    }}\n"
                        ));
                    }
                }
                tests.push_str("    return 1;\n");
                tests
            }
            Type::Named(named) => {
                let fields: Vec<String> = (self.struct_fields(named).into_iter())
                    .map(|(field, field_type, _)| match self.text_capacity(named, &field) {
                        Some(_) => format!(
                            "dn_text_equal(a->{field}.text, a->{field}.length, b->{field}.text, b->{field}.length)"
                        ),
                        None => self.compare(&format!("a->{field}"), &format!("b->{field}"), &field_type),
                    })
                    .collect();
                let all = if fields.is_empty() {
                    "1".to_string()
                } else {
                    fields.join("\n        && ")
                };
                format!("    return {all};\n")
            }
            _ => "    return 1;\n".to_string(),
        }
    }

    fn compare(&self, a: &str, b: &str, ty: &Type) -> String {
        if self.is_scalar(ty) {
            return format!("({a} == {b})");
        }
        if *ty == Type::String {
            return format!("dn_string_equal(&{a}, &{b})");
        }
        if *ty == Type::Bytes {
            return format!("dn_bytes_equal(&{a}, &{b})");
        }
        format!("{}(&{a}, &{b})", self.equality_for(ty))
    }
}
