//! C definitions for every type the program uses, each after the types it contains.

use std::collections::HashSet;

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;

use super::width::int_width;
use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    pub fn type_definitions(&self) -> String {
        let mut done = HashSet::new();
        let mut out = String::new();
        let mut index = 0;
        loop {
            let next = self.used_types.borrow().0.get(index).cloned();
            let Some(ty) = next else { break };
            self.define(&ty, &mut done, &mut out);
            index += 1;
        }
        let texts: String = (self.texts.borrow().iter())
            .map(|n| format!("typedef struct {{\n    dn_u16 length;\n    char text[{n}];\n}} dn_text_{n};\n\n"))
            .collect();
        texts + &out
    }

    fn define(&self, ty: &Type, done: &mut HashSet<Type>, out: &mut String) {
        let ty = self.canonical(ty);
        if !done.insert(ty.clone()) {
            return;
        }
        for dependency in self.dependencies(&ty) {
            self.define(&dependency, done, out);
        }
        out.push_str(&self.definition(&ty));
    }

    fn dependencies(&self, ty: &Type) -> Vec<Type> {
        match ty {
            Type::List(item) | Type::Option(item) => vec![(**item).clone()],
            Type::Map(key, value) => vec![(**key).clone(), (**value).clone()],
            Type::Result(value, error) => vec![(**value).clone(), (**error).clone()],
            Type::Errors(_) => self
                .errors()
                .into_iter()
                .flat_map(|(_, fields)| fields)
                .map(|(_, ty)| ty)
                .collect(),
            Type::Named(named) => match named.kind {
                SymbolKind::Enum => self
                    .model
                    .variants(named)
                    .into_iter()
                    .flat_map(|v| v.fields)
                    .map(|(_, ty)| ty)
                    .collect(),
                SymbolKind::Identity => Vec::new(),
                _ => self
                    .struct_fields(named)
                    .into_iter()
                    .map(|(_, ty, _)| ty)
                    .collect(),
            },
            _ => Vec::new(),
        }
    }

    fn definition(&self, ty: &Type) -> String {
        match ty {
            Type::Errors(_) => self.error_definition(),
            Type::List(_) | Type::Map(..) | Type::Option(_) | Type::Result(..) => {
                self.generic_definition(ty)
            }
            Type::Named(named) => match named.kind {
                SymbolKind::Identity => {
                    let scope = &self.program.contexts[&named.context];
                    let identity = scope.identities.iter().find(|i| i.name.name == named.name);
                    let constraints = identity.map_or(&[][..], |i| &i.constraints[..]);
                    format!(
                        "typedef {} {};\n\n",
                        int_width(self.program, &self.model, &named.context, constraints),
                        c_name(named)
                    )
                }
                SymbolKind::Enum => self.enum_definition(named),
                _ => {
                    let fields: Vec<String> = self
                        .struct_fields(named)
                        .into_iter()
                        .map(|(name, _, c)| format!("    {c} {name};\n"))
                        .collect();
                    struct_definition(&c_name(named), &fields.concat())
                }
            },
            _ => String::new(),
        }
    }
}

/// `typedef struct { ... } Name;`, with a placeholder field when there are none.
pub fn struct_definition(name: &str, fields: &str) -> String {
    let fields = if fields.is_empty() {
        "    dn_u8 unused;\n"
    } else {
        fields
    };
    format!("typedef struct {{\n{fields}}} {name};\n\n")
}
