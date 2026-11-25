//! The fields of the structs for values, entities, aggregates and events. A
//! string field with a `Length` shorter than the longest string anywhere is
//! kept in a `croft_text_N` of just that many characters, and copied into a
//! full `croft_string` when it is read.

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
}
