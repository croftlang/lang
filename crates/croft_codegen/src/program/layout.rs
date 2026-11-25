//! How big a saved value can be, and a description of its layout whose
//! hash marks every saved file, so a change of layout is noticed on loading.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;

use crate::Generator;

impl<'a> Generator<'a> {
    /// The most bytes a value of the type can take once written.
    pub fn serialized_size(&self, ty: &Type) -> i64 {
        let ty = self.canonical(ty);

        if self.is_scalar(&ty) {
            return 4;
        }

        match &ty {
            Type::String => 2 + self.sizes.string_capacity,
            Type::Text(length) => 2 + i64::from(*length),
            Type::Bytes => 2 + self.sizes.bytes_capacity,
            Type::List(item) => 2 + self.sizes.list_capacity(item) * self.serialized_size(item),
            Type::Option(item) => 4 + self.serialized_size(item),
            Type::Map(key, value) => {
                2 + self.sizes.map_capacity(&ty)
                    * (self.serialized_size(key) + self.serialized_size(value))
            }
            Type::Named(named) if named.kind == SymbolKind::Enum => {
                let variants = self.variants(named).into_iter().map(|variant| {
                    (variant.fields.iter())
                        .map(|(_, ty)| self.serialized_size(ty))
                        .sum::<i64>()
                });

                4 + variants.max().unwrap_or(0)
            }
            Type::Named(named) => {
                let fields: i64 = (self.struct_fields(named).into_iter())
                    .filter(|(field, _, _)| self.flag_bit(named, field).is_none())
                    .map(|(field, ty, _)| match self.text_capacity(named, &field) {
                        Some(capacity) => 2 + capacity,
                        None => self.serialized_size(&ty),
                    })
                    .sum();

                fields + self.flag_bytes(named) as i64
            }
            _ => 0,
        }
    }

    /// A description of how a type is laid out, which changes whenever its
    /// saved form would. Its hash goes at the start of every saved file.
    pub fn layout(&self, ty: &Type) -> String {
        let ty = self.canonical(ty);

        match &ty {
            Type::List(item) => format!("List<{}>", self.layout(item)),
            Type::Option(item) => format!("Option<{}>", self.layout(item)),
            Type::Map(key, value) => format!("Map<{},{}>", self.layout(key), self.layout(value)),
            Type::Named(named) if named.kind == SymbolKind::Enum && !self.is_scalar(&ty) => {
                let variants: Vec<String> = (self.variants(named).into_iter())
                    .map(|variant| {
                        let fields: Vec<String> = (variant.fields.iter())
                            .map(|(name, ty)| format!("{name}:{}", self.layout(ty)))
                            .collect();

                        format!("{}({})", variant.name, fields.join(","))
                    })
                    .collect();

                format!("{}{{{}}}", named.name, variants.join("|"))
            }
            Type::Named(named) if !self.is_scalar(&ty) => {
                let fields: Vec<String> = (self.struct_fields(named).into_iter())
                    .map(|(name, ty, _)| match self.text_capacity(named, &name) {
                        Some(capacity) => format!("{name}:String<{capacity}>"),
                        None if self.flag_bit(named, &name).is_some() => format!("{name}:Flag"),
                        None => format!("{name}:{}", self.layout(&ty)),
                    })
                    .collect();

                format!("{}{{{}}}", named.name, fields.join(","))
            }
            other => other.to_string(),
        }
    }
}

/// A 16-bit FNV-1a hash, small enough to check on an 8-bit machine.
pub fn layout_hash(layout: &str) -> u16 {
    let mut hash: u32 = 0x811c_9dc5;

    for byte in layout.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    ((hash >> 16) ^ (hash & 0xffff)) as u16
}
