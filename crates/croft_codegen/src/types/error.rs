//! Errors: every error in the program shares one C type, `croft_error`, which
//! holds a code and the fields of whichever error it is.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Type, VariantInfo};

use super::define::struct_definition;
use crate::names::{c_name, error_code};
use crate::Generator;

impl<'a> Generator<'a> {
    /// Every error, with its fields: declared errors, and the variants an
    /// error enum declares itself, such as `GameError.KeyboardUnavailable`.
    pub fn errors(&self) -> Vec<(Resolved, Vec<(String, Type)>)> {
        let mut errors: Vec<(Resolved, Vec<(String, Type)>)> =
            (self.model.all(SymbolKind::Error).into_iter())
                .map(|error| {
                    let fields = self
                        .model
                        .fields(&error)
                        .into_iter()
                        .map(|(name, ty, _)| (name, ty))
                        .collect();
                    (error, fields)
                })
                .collect();
        for enum_decl in self.model.all(SymbolKind::Enum) {
            if !self.model.is_error(&Type::Named(enum_decl.clone())) {
                continue;
            }
            for variant in self
                .model
                .variants(&enum_decl)
                .into_iter()
                .filter(|v| v.error.is_none())
            {
                errors.push((self.own_error(&enum_decl, &variant), variant.fields));
            }
        }
        errors
    }

    /// The error a variant of an error enum stands for.
    pub fn own_error(&self, owner: &Resolved, variant: &VariantInfo) -> Resolved {
        variant.error.clone().unwrap_or_else(|| Resolved {
            context: owner.context.clone(),
            name: format!("{}.{}", owner.name, variant.name),
            kind: SymbolKind::Error,
        })
    }

    pub fn error_definition(&self) -> String {
        let mut out = String::new();
        let mut cases = String::new();
        for (index, (error, fields)) in self.errors().iter().enumerate() {
            out.push_str(&format!("#define {} {}\n", error_code(error), index + 1));
            if fields.is_empty() {
                continue;
            }
            let fields: String = fields
                .iter()
                .map(|(f, ty)| format!("            {} {f};\n", self.c_type(ty)))
                .collect();
            cases.push_str(&format!(
                "        struct {{\n{fields}        }} {};\n",
                c_name(error)
            ));
        }
        let body = format!(
            "    croft_u16 code;\n    union {{\n{cases}        croft_u8 none;\n    }} fields;\n"
        );
        out.push('\n');
        out.push_str(&struct_definition("croft_error", &body));
        out
    }
}
