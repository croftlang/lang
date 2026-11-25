//! Parameters and C signatures of generated functions. Numbers are passed by
//! value, strings as a borrowed `croft_string_view`, everything else by
//! pointer, and every result through `out`.

use croft_analysis::Type;
use croft_syntax::ast::Parameter;

use crate::function::Writer;
use crate::Generator;

impl<'a> Generator<'a> {
    pub(super) fn lower_parameters(
        &self,
        context: &str,
        aggregate: Option<&str>,
        parameters: &[Parameter],
    ) -> Vec<(String, Type)> {
        let lower = |p: &Parameter| {
            p.types.first().map_or(Type::Unknown, |ty| {
                self.model.lower_type(context, aggregate, ty)
            })
        };

        parameters
            .iter()
            .map(|p| (p.name.name.clone(), lower(p)))
            .collect()
    }

    /// `void name(croft_int p_a, const Position *p_b, Result *out)`.
    pub fn signature(
        &self,
        name: &str,
        leading: &[String],
        parameters: &[(String, Type)],
        result: Option<&Type>,
    ) -> String {
        let mut list: Vec<String> = leading.to_vec();

        for (parameter, ty) in parameters {
            let c_type = self.c_type(ty);

            list.push(if *ty == Type::String {
                format!("croft_string_view p_{parameter}")
            } else if self.is_scalar(ty) {
                format!("{c_type} p_{parameter}")
            } else {
                format!("const {c_type} *p_{parameter}")
            });
        }

        if let Some(result) = result {
            list.push(format!("{} *out", self.c_type(result)));
        }

        let list = if list.is_empty() {
            "void".to_string()
        } else {
            list.join(", ")
        };

        format!("void {name}({list})")
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// Makes each parameter's Croft name refer to its C parameter.
    pub fn bind_parameters(&mut self, parameters: &[(String, Type)]) {
        for (name, ty) in parameters {
            let c = format!("p_{name}");
            self.parameter(&c);

            if *ty == Type::String {
                self.views.insert(c.clone());
            }
            let value = if self.gen.is_scalar(ty) || *ty == Type::String {
                c
            } else {
                format!("(*{c})")
            };

            self.bind(name, value, ty.clone());
        }
    }
}
