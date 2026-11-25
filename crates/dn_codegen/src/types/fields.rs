//! The fields of the structs for values, entities, aggregates and events.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::types::Resolved;
use dn_analysis::Type;

use super::width::int_width;
use crate::Generator;

impl<'a> Generator<'a> {
    /// A struct's fields: name, dn type and C type. Whole numbers with a
    /// `Range` get the smallest C type that fits. Events also carry `source`.
    pub fn struct_fields(&self, named: &Resolved) -> Vec<(String, Type, String)> {
        let mut fields: Vec<(String, Type, String)> = (self.model.fields(named).into_iter())
            .map(|(name, ty, field)| {
                let c = match ty {
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
}
