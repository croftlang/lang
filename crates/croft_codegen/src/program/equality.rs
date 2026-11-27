//! Equality for structs, lists, options and errors, written as functions on demand.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;

use crate::types::bounds::list_item;
use crate::Generator;

impl<'a> Generator<'a> {
    /// A C expression that is true when `a` and `b`, two values of `ty`,
    /// are equal. Lists are compared by their items and lengths, so lists of
    /// any size share one function for their item type.
    pub fn equality_call(&self, a: &str, b: &str, ty: &Type) -> String {
        let name = self.equality_for(ty);

        if list_item(ty).is_some() {
            return format!("{name}({a}.items, {a}.length, {b}.items, {b}.length)");
        }

        format!("{name}(&{a}, &{b})")
    }

    /// The name of the function that compares two values of `ty`, generated
    /// later. Every list of the same items shares one.
    fn equality_for(&self, ty: &Type) -> String {
        let ty = match list_item(ty) {
            Some(item) => Type::list(self.canonical(item)),
            None => self.canonical(ty),
        };

        let name = match &ty {
            Type::List(item) => format!("croft_equal_list_{}", self.short(item)),
            _ => format!(
                "croft_equal_{}",
                self.c_type(&ty).trim_start_matches("croft_")
            ),
        };

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

            let signature = match list_item(&ty) {
                Some(item) => {
                    let item = self.c_type(item);
                    format!(
                        "croft_bool {name}(const {item} *a, croft_u16 a_length, const {item} *b, croft_u16 b_length)"
                    )
                }
                None => {
                    let c_type = self.c_type(&ty);
                    format!("croft_bool {name}(const {c_type} *a, const {c_type} *b)")
                }
            };

            prototypes.push(format!("{signature};"));

            let body = self.equality_body(&ty);
            out.push_str(&format!("{signature}\n{{\n{body}}}\n\n"));
        }

        out
    }

    /// The body of the function that compares two values of `ty`.
    fn equality_body(&self, ty: &Type) -> String {
        match ty {
            Type::List(item) | Type::SizedList(item, _) => {
                let same = self.compare("a[index]", "b[index]", item);

                format!(
                    "    croft_u16 index;\n\n    if (a_length != b_length) {{\n        return 0;\n    }}\n    \
                     for (index = 0; index < a_length; index++) {{\n        if (!{same}) {{\n            return 0;\n        }}\n    }}\n    return 1;\n"
                )
            }
            // The same keys with the same values, in any order.
            Type::Map(key, value) => {
                let same_key = self.compare("a->keys[index]", "b->keys[other]", key);
                let same_value = self.compare("a->values[index]", "b->values[other]", value);

                format!(
                    "    croft_u16 index;\n    croft_u16 other;\n\n    if (a->length != b->length) {{\n        return 0;\n    }}\n    \
                     for (index = 0; index < a->length; index++) {{\n        \
                     for (other = 0; other < b->length && !{same_key}; other++) {{\n        }}\n        \
                     if (other == b->length || !{same_value}) {{\n            return 0;\n        }}\n    }}\n    return 1;\n"
                )
            }
            // The same items, in any order. A set holds each item once, so
            // finding each of `a`'s items in `b` is enough.
            Type::Set(item) => {
                let same = self.compare("a->items[index]", "b->items[other]", item);

                format!(
                    "    croft_u16 index;\n    croft_u16 other;\n\n    if (a->length != b->length) {{\n        return 0;\n    }}\n    \
                     for (index = 0; index < a->length; index++) {{\n        \
                     for (other = 0; other < b->length && !{same}; other++) {{\n        }}\n        \
                     if (other == b->length) {{\n            return 0;\n        }}\n    }}\n    return 1;\n"
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

                for variant in self.variants(named) {
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
                let flags = (0..self.flag_bytes(named))
                    .map(|byte| format!("a->croft_flags[{byte}] == b->croft_flags[{byte}]"));

                let fields: Vec<String> = (self.struct_fields(named).into_iter())
                    .filter(|(field, _, _)| self.flag_bit(named, field).is_none())
                    .map(
                        |(field, field_type, _)| match self.text_capacity(named, &field) {
                            Some(_) => format!(
                            "croft_string_equal(CROFT_VIEW(a->{field}), CROFT_VIEW(b->{field}))"
                        ),
                            None => self.compare(
                                &format!("a->{field}"),
                                &format!("b->{field}"),
                                &field_type,
                            ),
                        },
                    )
                    .chain(flags)
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

    /// A C expression that is true when `a` and `b`, two values of `ty`, are
    /// equal: `==` for numbers, a runtime call for strings and bytes, and an
    /// equality function for anything else.
    fn compare(&self, a: &str, b: &str, ty: &Type) -> String {
        if self.is_scalar(ty) {
            return format!("({a} == {b})");
        }

        if matches!(ty, Type::String | Type::Text(_)) {
            return format!("croft_string_equal(CROFT_VIEW({a}), CROFT_VIEW({b}))");
        }

        if matches!(ty, Type::Bytes | Type::SizedBytes(_)) {
            return format!("croft_bytes_equal(CROFT_BYTES_VIEW({a}), CROFT_BYTES_VIEW({b}))");
        }

        self.equality_call(a, b, ty)
    }
}
