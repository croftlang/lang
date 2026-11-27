//! How many bytes each C type takes, for `croft size`. Worked out the way
//! an 8-bit compiler lays them out: no padding, two-byte pointers, and four
//! bytes for a `croft_int`. Other machines pad, so treat it as a guide.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;

use super::width::int_width;
use crate::Generator;

/// A pointer on the Z80 and 6502.
pub const POINTER_SIZE: i64 = 2;

/// Bytes for the C integer types the generated code uses.
pub fn c_size(c_type: &str) -> Option<i64> {
    let size = match c_type {
        "croft_u8" | "croft_i8" | "croft_bool" | "croft_unit" => 1,
        "croft_u16" | "croft_i16" => 2,
        "croft_int" | "croft_i32" | "long" => 4,
        "double" => 8,
        "croft_string_view" => POINTER_SIZE + 2,
        _ => {
            let capacity = (c_type.strip_prefix("croft_text_"))
                .or_else(|| c_type.strip_prefix("croft_bytes_"))?
                .parse::<i64>()
                .ok()?;

            2 + capacity
        }
    };

    Some(size)
}

impl<'a> Generator<'a> {
    /// The bytes a value of `ty` takes in memory.
    pub fn memory_size(&self, ty: &Type) -> i64 {
        let ty = self.canonical(ty);

        match &ty {
            Type::Int | Type::Decimal(_) => 4,
            Type::Float => 8,
            Type::Bool | Type::Unit => 1,
            Type::String => 2 + self.sizes.string_capacity,
            Type::Text(length) => 2 + i64::from(*length),
            Type::Borrowed(_) => POINTER_SIZE + 2,
            Type::Bytes => 2 + self.sizes.bytes_capacity,
            Type::SizedBytes(length) => 2 + i64::from(*length),
            Type::List(item) | Type::SizedList(item, _) => {
                2 + self.capacity(&ty) * self.memory_size(item)
            }
            Type::Map(key, value) => {
                let map = Type::map((**key).clone(), (**value).clone());
                2 + self.sizes.map_capacity(&map)
                    * (self.memory_size(key) + self.memory_size(value))
            }
            Type::Set(item) => 2 + self.sizes.set_capacity(&ty) * self.memory_size(item),
            Type::Option(item) => 1 + self.memory_size(item),
            Type::Result(value, error) => 1 + self.memory_size(value) + self.memory_size(error),
            Type::Errors(_) => {
                let fields = (self.errors().into_iter())
                    .map(|(_, fields)| {
                        fields
                            .iter()
                            .map(|(_, ty)| self.memory_size(ty))
                            .sum::<i64>()
                    })
                    .max()
                    .unwrap_or(0);

                2 + fields.max(1)
            }
            Type::Named(named) => match named.kind {
                SymbolKind::Identity => {
                    let scope = &self.program.contexts[&named.context];
                    let identity = scope.identities.iter().find(|i| i.name.name == named.name);
                    let constraints = identity.map_or(&[][..], |i| &i.constraints[..]);

                    c_size(int_width(
                        self.program,
                        &self.model,
                        &named.context,
                        constraints,
                    ))
                    .unwrap_or(4)
                }
                SymbolKind::Enum if self.is_plain_enum(named) => 1,
                SymbolKind::Enum => {
                    let largest = (self.variants(named).into_iter())
                        .map(|v| {
                            v.fields
                                .iter()
                                .map(|(_, ty)| self.memory_size(ty))
                                .sum::<i64>()
                        })
                        .max()
                        .unwrap_or(0);

                    1 + largest.max(1)
                }
                _ => {
                    let fields: i64 = (self.struct_fields(named).into_iter())
                        .filter(|(name, _, _)| self.flag_bit(named, name).is_none())
                        .map(|(_, ty, c)| c_size(&c).unwrap_or_else(|| self.memory_size(&ty)))
                        .sum();

                    (fields + self.flag_bytes(named) as i64).max(1)
                }
            },
            Type::Port(_) | Type::Repository(_) | Type::Unknown => 1,
        }
    }

    /// The bytes of a C type by its name, for any type this program uses.
    pub fn c_type_size(&self, c_type: &str) -> Option<i64> {
        if let Some(size) = c_size(c_type) {
            return Some(size);
        }

        let used = self.used_types.borrow().0.clone();

        used.iter()
            .find(|ty| self.c_type(ty) == c_type)
            .map(|ty| self.memory_size(ty))
    }
}
