//! Turning values into bytes and back, for `FileStorage`. Every number is
//! four bytes, least significant first, whatever width it has in memory, so
//! a save means the same on every machine. Strings, bytes and lists start
//! with their length in two bytes; an option starts with whether it is set.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;

use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The name part of `dn_write_X` and `dn_read_X` for a type, asking for
    /// both functions to be written.
    pub fn serializer_for(&self, ty: &Type) -> String {
        let ty = self.canonical(ty);
        let name = self.c_type(&ty).trim_start_matches("dn_").to_string();
        let mut serialized = self.serialized.borrow_mut();
        if serialized.1.insert(ty.clone()) {
            serialized.0.push(ty);
        }
        name
    }

    /// Every serializer asked for, including those they need in turn.
    pub fn serializer_functions(&self) -> String {
        let (mut prototypes, mut bodies) = (String::new(), String::new());
        let mut index = 0;
        loop {
            let next = self.serialized.borrow().0.get(index).cloned();
            let Some(ty) = next else {
                break;
            };
            index += 1;
            let name = self.serializer_for(&ty);
            let c = self.c_type(&ty);
            let write = format!("static dn_bool dn_write_{name}(const {c} *value, dn_bytes *out)");
            let read = format!(
                "static dn_bool dn_read_{name}(const dn_bytes *in, dn_u16 *at, {c} *value)"
            );
            prototypes.push_str(&format!("{write};\n{read};\n"));
            let (write_body, read_body) = self.serializer_bodies(&ty);
            bodies.push_str(&format!(
                "{write}\n{{\n{write_body}    return 1;\n}}\n\n{read}\n{{\n    dn_int number = 0;\n\n{read_body}    (void) number;\n    return 1;\n}}\n\n"
            ));
        }
        if prototypes.is_empty() {
            return String::new();
        }
        format!("{prototypes}\n{bodies}")
    }

    fn serializer_bodies(&self, ty: &Type) -> (String, String) {
        match ty {
            Type::List(item) => {
                let capacity = self.sizes.list_capacity(item);
                let (write, read) = self.serialize_value("value->items[index]", item, "        ");
                (
                    format!(
                        "    dn_u16 index;\n\n    if (!dn_bytes_put_length(out, value->length)) {{\n        return 0;\n    }}\n    \
                         for (index = 0; index < value->length; index++) {{\n{write}    }}\n"
                    ),
                    format!(
                        "    dn_u16 index;\n\n    if (!dn_bytes_take_length(in, at, &value->length) || value->length > {capacity}) {{\n        return 0;\n    }}\n    \
                         for (index = 0; index < value->length; index++) {{\n{read}    }}\n"
                    ),
                )
            }
            Type::Map(key, value) => {
                let capacity = self.sizes.map_capacity(ty);
                let (key_write, key_read) =
                    self.serialize_value("value->keys[index]", key, "        ");
                let (value_write, value_read) =
                    self.serialize_value("value->values[index]", value, "        ");
                (
                    format!(
                        "    dn_u16 index;\n\n    if (!dn_bytes_put_length(out, value->length)) {{\n        return 0;\n    }}\n    \
                         for (index = 0; index < value->length; index++) {{\n{key_write}{value_write}    }}\n"
                    ),
                    format!(
                        "    dn_u16 index;\n\n    if (!dn_bytes_take_length(in, at, &value->length) || value->length > {capacity}) {{\n        return 0;\n    }}\n    \
                         for (index = 0; index < value->length; index++) {{\n{key_read}{value_read}    }}\n"
                    ),
                )
            }
            Type::Option(item) => {
                let (write, read) = self.serialize_value("value->value", item, "        ");
                (
                    format!(
                        "    if (!dn_bytes_put_int(out, value->some)) {{\n        return 0;\n    }}\n    if (value->some) {{\n{write}    }}\n"
                    ),
                    format!(
                        "    if (!dn_bytes_take_int(in, at, &number)) {{\n        return 0;\n    }}\n    value->some = number != 0;\n    \
                         if (value->some) {{\n{read}    }}\n"
                    ),
                )
            }
            Type::Named(named) if named.kind == SymbolKind::Enum => {
                let (mut write, mut read) = (
                    "    if (!dn_bytes_put_int(out, value->tag)) {\n        return 0;\n    }\n".to_string(),
                    "    if (!dn_bytes_take_int(in, at, &number)) {\n        return 0;\n    }\n    value->tag = number;\n".to_string(),
                );
                for variant in self.model.variants(named) {
                    let tag = format!("{}_{}", c_name(named), variant.name);
                    let (mut fields_write, mut fields_read) = (String::new(), String::new());
                    for (field, field_type) in &variant.fields {
                        let place = format!("value->as.{}.{field}", variant.name);
                        let (w, r) = self.serialize_value(&place, field_type, "        ");
                        fields_write.push_str(&w);
                        fields_read.push_str(&r);
                    }
                    if !fields_write.is_empty() {
                        write.push_str(&format!(
                            "    if (value->tag == {tag}) {{\n{fields_write}    }}\n"
                        ));
                        read.push_str(&format!(
                            "    if (value->tag == {tag}) {{\n{fields_read}    }}\n"
                        ));
                    }
                }
                (write, read)
            }
            Type::Named(named) => {
                let (mut write, mut read) = (String::new(), String::new());
                for (field, field_type, _) in self.struct_fields(named) {
                    let place = format!("value->{field}");
                    if let Some(capacity) = self.text_capacity(named, &field) {
                        write.push_str(&format!(
                            "    if (!dn_bytes_put_text(out, {place}.text, {place}.length)) {{\n        return 0;\n    }}\n"
                        ));
                        read.push_str(&format!(
                            "    if (!dn_bytes_take_text(in, at, {place}.text, &{place}.length, {capacity})) {{\n        return 0;\n    }}\n"
                        ));
                        continue;
                    }
                    let (w, r) = self.serialize_value(&place, &field_type, "    ");
                    write.push_str(&w);
                    read.push_str(&r);
                }
                (write, read)
            }
            _ => (String::new(), String::new()),
        }
    }

    /// Statements that write the value at `place`, and read it back.
    fn serialize_value(&self, place: &str, ty: &Type, indent: &str) -> (String, String) {
        let fail = format!(" {{\n{indent}    return 0;\n{indent}}}\n");
        if self.is_scalar(ty) {
            return (
                format!("{indent}if (!dn_bytes_put_int(out, (dn_int) {place})){fail}"),
                format!("{indent}if (!dn_bytes_take_int(in, at, &number)){fail}{indent}{place} = number;\n"),
            );
        }
        let name = match ty {
            Type::String => "string".to_string(),
            Type::Bytes => "bytes".to_string(),
            _ => self.serializer_for(ty),
        };
        let (write, read) = match ty {
            Type::String | Type::Bytes => (
                format!("dn_bytes_put_{name}(out, &{place})"),
                format!("dn_bytes_take_{name}(in, at, &{place})"),
            ),
            _ => (
                format!("dn_write_{name}(&{place}, out)"),
                format!("dn_read_{name}(in, at, &{place})"),
            ),
        };
        (
            format!("{indent}if (!{write}){fail}"),
            format!("{indent}if (!{read}){fail}"),
        )
    }
}
