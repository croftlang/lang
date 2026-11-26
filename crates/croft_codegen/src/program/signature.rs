//! Parameters and C signatures of generated functions. Numbers are passed by
//! value, strings as a borrowed `croft_string_view`, everything else by
//! pointer, and every result through `out`.

use croft_analysis::Type;
use croft_syntax::ast::{ConstraintUse, Parameter};

use crate::function::range::Span;
use crate::function::Writer;
use crate::types::bounds::is_string;
use crate::types::width::{declared_range, width_for};
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
            .map(|p| {
                let ty = self.parameter_type(context, lower(p), &p.constraints);
                (p.name.name.clone(), ty)
            })
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
        self.ranged_signature(name, leading, parameters, &[], result)
    }

    /// A signature in which each whole number with a known range, in
    /// `ranges`, is passed in the smallest type that holds it. Only Croft
    /// calls these functions, so native code never sees the narrow types.
    pub fn ranged_signature(
        &self,
        name: &str,
        leading: &[String],
        parameters: &[(String, Type)],
        ranges: &[Option<Span>],
        result: Option<&Type>,
    ) -> String {
        let mut list: Vec<String> = leading.to_vec();

        for (index, (parameter, ty)) in parameters.iter().enumerate() {
            let c_type = match ranges.get(index).copied().flatten() {
                Some(range) if width_for(range.low, range.high) != "croft_int" => {
                    format!("CROFT_RANGED({})", width_for(range.low, range.high))
                }
                _ => self.c_type(ty),
            };

            list.push(if is_string(ty) {
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

impl<'a> Generator<'a> {
    /// The values each whole-number parameter can be, from its `Range`.
    pub fn parameter_ranges(
        &self,
        context: &str,
        parameters: &[(String, Type)],
        constraints: &[&[ConstraintUse]],
    ) -> Vec<Option<Span>> {
        (parameters.iter().zip(constraints))
            .map(|((_, ty), constraints)| {
                if *ty != Type::Int {
                    return None;
                }

                let (low, high) = declared_range(self.program, &self.model, context, constraints)?;

                Some(Span::new(low, high))
            })
            .collect()
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// Records the values each whole-number parameter can be.
    pub fn note_parameter_ranges(
        &mut self,
        parameters: &[(String, Type)],
        ranges: &[Option<Span>],
    ) {
        for ((name, _), range) in parameters.iter().zip(ranges) {
            if let Some(range) = range {
                self.note_range(&format!("p_{name}"), *range);
            }
        }
    }

    /// Makes each parameter's Croft name refer to its C parameter.
    pub fn bind_parameters(&mut self, parameters: &[(String, Type)]) {
        for (name, ty) in parameters {
            let c = format!("p_{name}");
            self.parameter(&c);

            if is_string(ty) {
                self.views.insert(c.clone());
            }

            let value = if self.gen.is_scalar(ty) || is_string(ty) {
                c
            } else {
                format!("(*{c})")
            };

            self.bind(name, value, ty.clone());
        }
    }
}
