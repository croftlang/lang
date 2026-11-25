//! Sizes taken from constraints: how long strings and lists can be, and how many
//! bytes a whole number needs.

use std::collections::HashMap;

use croft_analysis::{Model, Program, Type};
use croft_syntax::ast::{ConstraintUse, TypeExpr};

use super::constant::evaluate;

/// Used when no `Length` constraint says otherwise.
pub const DEFAULT_LIST_CAPACITY: i64 = 16;
pub const DEFAULT_STRING_CAPACITY: i64 = 32;
pub const DEFAULT_BYTES_CAPACITY: i64 = 64;

pub struct Sizes {
    /// The room every working string has: the default, or the longest
    /// string any constraint allows if that is longer. A string field with a
    /// shorter `Length` is kept in a size of its own.
    pub string_capacity: i64,
    /// The most bytes any constraint allows. Every `Bytes` has room for them.
    pub bytes_capacity: i64,
    /// The longest list of each item type any constraint allows.
    list_capacity: HashMap<Type, i64>,
    /// The most entries each type of map can hold.
    map_capacity: HashMap<Type, i64>,
}

impl Sizes {
    pub fn measure(program: &Program, model: &Model) -> Sizes {
        let mut sizes = Sizes {
            string_capacity: 0,
            bytes_capacity: 0,
            list_capacity: HashMap::new(),
            map_capacity: HashMap::new(),
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
                .chain(context.ports.iter().flat_map(|p| &p.operations))
                .map(|f| (f, None));

            let commands = context.aggregates.iter().flat_map(|a| {
                let scope = Some(a.name.name.as_str());
                a.commands
                    .iter()
                    .chain(&a.factories)
                    .map(move |c| (c, scope))
            });

            // A `where` on a parameter or on the result sizes the list as
            // much as one on a field does.
            for (function, aggregate) in functions.chain(commands) {
                for parameter in function.parameters.iter().flatten() {
                    if let Some(ty) = parameter.types.first() {
                        fields.push((ty, &parameter.constraints, aggregate));
                    }
                }

                if let Some(ty) = &function.return_type {
                    fields.push((ty, &function.return_constraints, aggregate));
                }
            }

            for (ty, constraints, aggregate) in fields {
                sizes.add(program, model, name, aggregate, ty, constraints);
            }
        }

        if sizes.bytes_capacity == 0 {
            sizes.bytes_capacity = DEFAULT_BYTES_CAPACITY;
        }

        sizes.string_capacity = sizes.string_capacity.max(DEFAULT_STRING_CAPACITY);

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
            "Bytes" => self.bytes_capacity = self.bytes_capacity.max(longest),
            "List" => {
                let Some(item) = ty.arguments.first() else {
                    return;
                };

                let item = model.lower_type(context, aggregate, item);
                let capacity = self.list_capacity.entry(item).or_insert(0);
                *capacity = (*capacity).max(longest);
            }
            "Map" => {
                let [key, value] = ty.arguments.as_slice() else {
                    return;
                };

                let key = model.lower_type(context, aggregate, key);
                let value = model.lower_type(context, aggregate, value);
                let map = Type::map(key.clone(), value.clone());
                let capacity = self.map_capacity.entry(map).or_insert(0);
                *capacity = (*capacity).max(longest);

                // `keys` and `values` give lists, which need room for every entry.
                for item in [key, value] {
                    let list = self
                        .list_capacity
                        .entry(item)
                        .or_insert(DEFAULT_LIST_CAPACITY);

                    *list = (*list).max(longest);
                }
            }
            _ => {}
        }
    }

    /// The most entries a map of this type can hold.
    pub fn map_capacity(&self, map: &Type) -> i64 {
        self.map_capacity
            .get(map)
            .copied()
            .unwrap_or(DEFAULT_LIST_CAPACITY)
    }

    pub fn list_capacity(&self, item: &Type) -> i64 {
        self.list_capacity
            .get(item)
            .copied()
            .unwrap_or(DEFAULT_LIST_CAPACITY)
    }
}
