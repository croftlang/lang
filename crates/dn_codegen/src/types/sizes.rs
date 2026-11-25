//! Sizes taken from constraints: how long strings and lists can be, and how many
//! bytes a whole number needs.

use std::collections::HashMap;

use dn_analysis::{Model, Program, Type};
use dn_syntax::ast::{ConstraintUse, TypeExpr};

use super::constant::evaluate;

/// Used when no `Length` constraint says otherwise.
pub const DEFAULT_LIST_CAPACITY: i64 = 16;
pub const DEFAULT_STRING_CAPACITY: i64 = 32;

pub struct Sizes {
    /// The longest string any constraint allows. Every string has room for it.
    pub string_capacity: i64,
    /// The longest list of each item type any constraint allows.
    list_capacity: HashMap<Type, i64>,
}

impl Sizes {
    pub fn measure(program: &Program, model: &Model) -> Sizes {
        let mut sizes = Sizes {
            string_capacity: 0,
            list_capacity: HashMap::new(),
        };
        for (name, context) in &program.contexts {
            let mut fields: Vec<(&TypeExpr, &[ConstraintUse], Option<&str>)> = Vec::new();
            let records = context
                .values
                .iter()
                .map(|v| (&v.fields, None))
                .chain(context.errors.iter().map(|e| (&e.fields, None)));
            let records = records.chain(context.adapters.iter().map(|a| (&a.fields, None)));
            let records = records.chain(
                context
                    .enums
                    .iter()
                    .flat_map(|e| e.variants.iter().map(|v| (&v.fields, None))),
            );
            let aggregates = context.aggregates.iter().flat_map(|a| {
                let scope = Some(a.name.name.as_str());
                let nested = a.entities.iter().map(move |e| (&e.fields, scope));
                let records = a
                    .errors
                    .iter()
                    .chain(&a.events)
                    .map(move |r| (&r.fields, scope));
                std::iter::once((&a.fields, scope))
                    .chain(nested)
                    .chain(records)
            });
            for (list, aggregate) in records.chain(aggregates) {
                fields.extend(
                    list.iter()
                        .map(|f| (&f.ty, f.constraints.as_slice(), aggregate)),
                );
            }
            let functions = context
                .functions
                .iter()
                .chain(context.ports.iter().flat_map(|p| &p.operations));
            for parameter in functions.flat_map(|f| f.parameters.iter().flatten()) {
                if let Some(ty) = parameter.types.first() {
                    fields.push((ty, &parameter.constraints, None));
                }
            }
            for (ty, constraints, aggregate) in fields {
                sizes.add(program, model, name, aggregate, ty, constraints);
            }
        }
        if sizes.string_capacity == 0 {
            sizes.string_capacity = DEFAULT_STRING_CAPACITY;
        }
        sizes
    }

    fn add(
        &mut self,
        program: &Program,
        model: &Model,
        context: &str,
        aggregate: Option<&str>,
        ty: &TypeExpr,
        constraints: &[ConstraintUse],
    ) {
        let Some(longest) = constraints
            .iter()
            .filter(|r| !r.each && r.name.text() == "Length")
            .find_map(|constraint| {
                let high = constraint.arguments.last()?;
                evaluate(program, model, context, high)
            })
        else {
            return;
        };
        match ty.path.text().as_str() {
            "String" => self.string_capacity = self.string_capacity.max(longest),
            "List" => {
                let Some(item) = ty.arguments.first() else {
                    return;
                };
                let item = model.lower_type(context, aggregate, item);
                let capacity = self.list_capacity.entry(item).or_insert(0);
                *capacity = (*capacity).max(longest);
            }
            _ => {}
        }
    }

    pub fn list_capacity(&self, item: &Type) -> i64 {
        self.list_capacity
            .get(item)
            .copied()
            .unwrap_or(DEFAULT_LIST_CAPACITY)
    }
}
