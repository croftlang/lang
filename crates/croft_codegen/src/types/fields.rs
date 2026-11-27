//! The fields of the structs for values, entities, aggregates and events. A
//! string field with a `Length` shorter than the longest string anywhere is
//! kept in a `croft_text_N` of just that many characters, a `Bytes` field
//! with one other than the target's `bytes_length` in a `croft_bytes_N`, and a list
//! field with one shorter than the longest list of its items in a
//! `croft_listN_...`. When a value, entity or aggregate has two or more `Bool`
//! fields, each is one bit of `croft_flags`, eight to a byte.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Type, VariantInfo};
use croft_syntax::ast::ConstraintUse;

use super::bounds::list_item;
use super::constant::evaluate;
use super::width::int_width;
use crate::Generator;

impl<'a> Generator<'a> {
    /// A struct's fields: name, Croft type and C type. Whole numbers with a
    /// `Range` get the smallest C type that fits, and a list kept in its own
    /// size has that size in its type. Events also carry `source`.
    pub fn struct_fields(&self, named: &Resolved) -> Vec<(String, Type, String)> {
        let mut fields: Vec<(String, Type, String)> = (self.model.fields(named).into_iter())
            .map(|(name, ty, field)| {
                let kept = self.own_size(named, &name);

                let c = match &kept {
                    Some(kept) => self.c_type(kept),
                    None => match ty {
                        Type::Int => int_width(
                            self.program,
                            &self.model,
                            &named.context,
                            &field.constraints,
                        )
                        .to_string(),
                        _ => self.c_type(&ty),
                    },
                };

                let ty = match kept {
                    Some(kept @ Type::SizedList(..)) => kept,
                    _ => ty,
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
        self.kept_capacity(owner, field, &Type::String)
    }

    /// How many values a `Bytes` field keeps, when that is more or fewer
    /// than a `Bytes` with no `Length` holds: its own `Length`.
    pub fn bytes_field_capacity(&self, owner: &Resolved, field: &str) -> Option<i64> {
        self.kept_capacity(owner, field, &Type::Bytes)
    }

    /// The type a field is kept in when it has a size of its own: a
    /// `croft_text_N`, a `croft_bytes_N` or a `croft_listN_...`.
    pub fn own_size(&self, owner: &Resolved, field: &str) -> Option<Type> {
        if let Some(capacity) = self.text_capacity(owner, field) {
            return Some(self.text_type(capacity));
        }

        if let Some(capacity) = self.bytes_field_capacity(owner, field) {
            return Some(self.bytes_type(capacity));
        }

        self.list_field_type(owner, field)
    }

    /// The type a list field is kept in when its `Length` allows fewer items
    /// than every other list of its items holds.
    fn list_field_type(&self, owner: &Resolved, field: &str) -> Option<Type> {
        if !self.keeps_own_sizes(owner) {
            return None;
        }

        let fields = self.model.fields(owner);
        let (_, ty, declared) = fields.into_iter().find(|(name, _, _)| name == field)?;
        let item = list_item(&ty)?.clone();
        let capacity = self.length_bound(&owner.context, &declared.constraints)?;

        match self.list_type(item, capacity) {
            sized @ Type::SizedList(..) => Some(sized),
            _ => None,
        }
    }

    /// Whether `owner` keeps fields in sizes of their own: values, entities,
    /// aggregates and events do, but errors and adapters keep full ones.
    fn keeps_own_sizes(&self, owner: &Resolved) -> bool {
        use SymbolKind::{Aggregate, Entity, Event, Value};

        matches!(owner.kind, Value | Entity | Aggregate | Event)
            && !self.model.is_error(&Type::Named(owner.clone()))
    }

    /// The `Length` of a field of type `kind`, `String` or `Bytes`, when it
    /// differs from the shared size for that type: below it for a string,
    /// which is as long as the longest, and either way for `Bytes`, whose
    /// shared size is the target's `bytes_length`.
    fn kept_capacity(&self, owner: &Resolved, field: &str, kind: &Type) -> Option<i64> {
        if !self.keeps_own_sizes(owner) {
            return None;
        }

        let fields = self.model.fields(owner);
        let (_, ty, declared) = fields.into_iter().find(|(name, _, _)| name == field)?;

        if ty != *kind {
            return None;
        }

        let capacity = self
            .length_bound(&owner.context, &declared.constraints)?
            .max(1);

        match kind {
            Type::String => (capacity < self.sizes.string_capacity).then_some(capacity),
            _ => (capacity != self.sizes.bytes_capacity).then_some(capacity),
        }
    }

    /// The most a `Length` among `constraints` allows, when there is one.
    pub fn length_bound(&self, context: &str, constraints: &[ConstraintUse]) -> Option<i64> {
        let length = constraints
            .iter()
            .find(|c| !c.each && c.name.text() == "Length")?;

        let high = length.arguments.last()?;

        evaluate(self.program, &self.model, context, high)
    }

    /// The variants of an enum. A string field with a `Length` shorter than
    /// every other string is kept in its own size, as on a value. Errors
    /// keep full strings.
    pub fn variants(&self, named: &Resolved) -> Vec<VariantInfo> {
        let mut variants = self.model.variants(named);

        if self.model.is_error(&Type::Named(named.clone())) {
            return variants;
        }

        for variant in &mut variants {
            let declared = &variant.declared_in;

            if variant.error.is_some() || declared.kind != SymbolKind::Enum {
                continue;
            }

            let record = (self.program.contexts.get(&declared.context))
                .and_then(|context| context.enums.iter().find(|e| e.name.name == declared.name))
                .and_then(|decl| decl.variants.iter().find(|v| v.name.name == variant.name));

            let Some(record) = record else { continue };

            for (name, ty) in &mut variant.fields {
                let Some(field) = record.fields.iter().find(|f| f.name.name == *name) else {
                    continue;
                };

                *ty = self.parameter_type(&declared.context, ty.clone(), &field.constraints);
            }
        }

        variants
    }

    /// A string, `Bytes` or list parameter's type: bounded by its `Length`,
    /// when it has one.
    pub fn parameter_type(&self, context: &str, ty: Type, constraints: &[ConstraintUse]) -> Type {
        match (ty, self.length_bound(context, constraints)) {
            (Type::String, Some(length)) => self.text_type(length),
            (Type::Bytes, Some(length)) => self.bytes_type(length),
            (Type::List(item), Some(length)) => self.list_type(*item, length),
            (ty, _) => ty,
        }
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
