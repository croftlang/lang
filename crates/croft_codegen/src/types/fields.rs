//! The fields of the structs for values, entities, aggregates and events. A
//! string field with a `Length` shorter than the longest string anywhere is
//! kept in a `croft_text_N` of just that many characters, and copied into a
//! full `croft_string` when it is read. When a value, entity or aggregate has
//! two or more `Bool` fields, each is one bit of `croft_flags`, eight to a byte.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;

use super::constant::evaluate;
use super::width::int_width;
use crate::Generator;

impl<'a> Generator<'a> {
    /// A struct's fields: name, Croft type and C type. Whole numbers with a
    /// `Range` get the smallest C type that fits. Events also carry `source`.
    pub fn struct_fields(&self, named: &Resolved) -> Vec<(String, Type, String)> {
        let mut fields: Vec<(String, Type, String)> = (self.model.fields(named).into_iter())
            .map(|(name, ty, field)| {
                let text = self.text_capacity(named, &name);

                let c = match ty {
                    Type::String if text.is_some() => {
                        let capacity = text.unwrap_or_default();
                        self.texts.borrow_mut().insert(capacity);
                        format!("croft_text_{capacity}")
                    }
                    Type::Int => int_width(
                        self.program,
                        &self.model,
                        &named.context,
                        &field.constraints,
                    )
                    .to_string(),
                    _ => self.c_type(&ty),
                };

                (name, ty, c)
            })
            .collect();

        if named.kind == SymbolKind::Event {
            let aggregate = named.name.split('.').next().unwrap_or_default();

            if let Some(aggregate) = self.model.lookup(&named.context, aggregate) {
                let identity = self.model.identity_of(&aggregate);

                fields.push((
                    "source".to_string(),
                    identity.clone(),
                    self.c_type(&identity),
                ));
            }
        }

        fields
    }

    /// How many characters a string field keeps, when that is fewer than
    /// every other string: its own `Length`. Errors, enum variants and
    /// adapters keep full strings.
    pub fn text_capacity(&self, owner: &Resolved, field: &str) -> Option<i64> {
        use SymbolKind::{Aggregate, Entity, Event, Value};

        if !matches!(owner.kind, Value | Entity | Aggregate | Event)
            || self.model.is_error(&Type::Named(owner.clone()))
        {
            return None;
        }

        let fields = self.model.fields(owner);
        let (_, ty, declared) = fields.into_iter().find(|(name, _, _)| name == field)?;

        if ty != Type::String {
            return None;
        }

        let length = declared
            .constraints
            .iter()
            .find(|c| !c.each && c.name.text() == "Length")?;

        let high = length.arguments.last()?;
        let capacity = evaluate(self.program, &self.model, &owner.context, high)?;

        (capacity < self.sizes.string_capacity).then_some(capacity.max(1))
    }

    /// Where a `Bool` field is kept when its owner's flags are packed: the
    /// byte of `croft_flags`, and the mask for its bit.
    pub fn flag_bit(&self, owner: &Resolved, field: &str) -> Option<(usize, u8)> {
        let flags = self.flag_fields(owner);
        let index = flags.iter().position(|name| name == field)?;
        Some((index / 8, 1 << (index % 8)))
    }

    /// How many bytes of `croft_flags` an owner has: none unless it packs.
    pub fn flag_bytes(&self, owner: &Resolved) -> usize {
        self.flag_fields(owner).len().div_ceil(8)
    }

    /// The `Bool` fields packed into bits, in order: those of a value,
    /// entity or aggregate with at least two of them.
    fn flag_fields(&self, owner: &Resolved) -> Vec<String> {
        use SymbolKind::{Aggregate, Entity, Value};
        if !matches!(owner.kind, Value | Entity | Aggregate)
            || self.model.is_error(&Type::Named(owner.clone()))
        {
            return Vec::new();
        }
        let flags: Vec<String> = (self.model.fields(owner).into_iter())
            .filter(|(_, ty, _)| *ty == Type::Bool)
            .map(|(name, _, _)| name)
            .collect();
        if flags.len() < 2 {
            return Vec::new();
        }
        flags
    }
}
