//! Sizes taken from constraints: how long strings and lists can be, and how many
//! bytes a whole number needs.

use std::collections::HashMap;

use croft_analysis::{Model, Program, Type};
use croft_syntax::ast::{ConstraintUse, TypeExpr};

use super::bounds::unbounded;
use super::constant::evaluate;

/// How many items a list or map holds when no `Length` constraint says otherwise.
pub const DEFAULT_LIST_CAPACITY: i64 = 16;
/// How many characters a string holds when no `Length` constraint says otherwise.
pub const DEFAULT_STRING_CAPACITY: i64 = 32;
/// How many bytes a `Bytes` holds when no `Length` constraint says otherwise.
pub const DEFAULT_BYTES_CAPACITY: i64 = 64;

/// The capacities of strings, bytes, lists and maps for one program.
pub struct Sizes {
    /// How many characters a string with no size of its own holds: the default, or the longest
    /// string any constraint allows if that is longer. A string field with a
    /// shorter `Length` is kept in a size of its own.
    pub string_capacity: i64,
    /// What set `string_capacity`, when a constraint did: a field or a
    /// parameter, such as `Terminal.print: text`.
    pub string_capacity_from: Option<String>,
    /// The most bytes any constraint allows, or the default. Every `Bytes`
    /// has room for them.
    pub bytes_capacity: i64,
    /// The longest list of each item type any constraint allows.
    list_capacity: HashMap<Type, i64>,
    /// The most entries each type of map can hold.
    map_capacity: HashMap<Type, i64>,
}

impl Sizes {
    /// Reads every `Length` in the program to work out its sizes.
    pub fn measure(program: &Program, model: &Model) -> Sizes {
        let mut sizes = Sizes {
            string_capacity: 0,
            string_capacity_from: None,
            bytes_capacity: 0,
            list_capacity: HashMap::new(),
            map_capacity: HashMap::new(),
        };

        for (name, context) in &program.contexts {
            let mut fields: Vec<(&TypeExpr, &[ConstraintUse], Option<&str>, String)> = Vec::new();

            let records = (context.values.iter())
                .map(|v| (&v.fields, None, v.name.name.clone()))
                .chain(
                    context
                        .errors
                        .iter()
                        .map(|e| (&e.fields, None, e.name.name.clone())),
                );

            let records = records
                .chain((context.adapters.iter()).map(|a| (&a.fields, None, a.name.name.clone())));

            let records = records.chain(context.enums.iter().flat_map(|e| {
                (e.variants.iter())
                    .map(move |v| (&v.fields, None, format!("{}.{}", e.name.name, v.name.name)))
            }));

            let aggregates = context.aggregates.iter().flat_map(|a| {
                let scope = Some(a.name.name.as_str());
                let owner = a.name.name.clone();
                let nested = (a.entities.iter())
                    .map(move |e| (&e.fields, scope, format!("{}.{}", a.name.name, e.name.name)));

                let records = (a.errors.iter().chain(&a.events))
                    .map(move |r| (&r.fields, scope, format!("{}.{}", a.name.name, r.name.name)));

                std::iter::once((&a.fields, scope, owner))
                    .chain(nested)
                    .chain(records)
            });

            for (list, aggregate, owner) in records.chain(aggregates) {
                fields.extend(list.iter().map(|f| {
                    let label = format!("{owner}.{}", f.name.name);
                    (&f.ty, f.constraints.as_slice(), aggregate, label)
                }));
            }

            let functions = (context.functions.iter())
                .map(|f| (f, None, f.name.name.clone()))
                .chain(context.ports.iter().flat_map(|p| {
                    (p.operations.iter())
                        .map(move |f| (f, None, format!("{}.{}", p.name.name, f.name.name)))
                }));

            let commands = context.aggregates.iter().flat_map(|a| {
                let scope = Some(a.name.name.as_str());
                (a.commands.iter().chain(&a.factories))
                    .map(move |c| (c, scope, format!("{}.{}", a.name.name, c.name.name)))
            });

            // A `where` on a parameter or on the result counts as much as
            // one on a field does.
            for (function, aggregate, owner) in functions.chain(commands) {
                for parameter in function.parameters.iter().flatten() {
                    if let Some(ty) = parameter.types.first() {
                        let label = format!("{owner}: {}", parameter.name.name);
                        fields.push((ty, &parameter.constraints, aggregate, label));
                    }
                }

                if let Some(ty) = &function.return_type {
                    let label = format!("{owner}'s result");
                    fields.push((ty, &function.return_constraints, aggregate, label));
                }
            }

            for (ty, constraints, aggregate, label) in fields {
                sizes.add(program, model, name, aggregate, ty, constraints, label);
            }
        }

        if sizes.bytes_capacity == 0 {
            sizes.bytes_capacity = DEFAULT_BYTES_CAPACITY;
        }

        sizes.string_capacity = sizes.string_capacity.max(DEFAULT_STRING_CAPACITY);

        sizes
    }

    /// Grows the capacity for one field, parameter or result by its `Length`.
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        program: &Program,
        model: &Model,
        context: &str,
        aggregate: Option<&str>,
        ty: &TypeExpr,
        constraints: &[ConstraintUse],
        label: String,
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
            "String" => {
                if longest > self.string_capacity.max(DEFAULT_STRING_CAPACITY) {
                    self.string_capacity_from = Some(label);
                }

                self.string_capacity = self.string_capacity.max(longest);
            }
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
            .get(&unbounded(map))
            .copied()
            .unwrap_or(DEFAULT_LIST_CAPACITY)
    }

    /// The most items a list of this item type can hold.
    pub fn list_capacity(&self, item: &Type) -> i64 {
        self.list_capacity
            .get(&unbounded(item))
            .copied()
            .unwrap_or(DEFAULT_LIST_CAPACITY)
    }
}
