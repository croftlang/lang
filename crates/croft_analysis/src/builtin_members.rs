//! The fields and methods the language gives its own types, by name, for
//! editors to offer. The checks in `check/body` (`property.rs`,
//! `method.rs`, `string.rs`, `list.rs`, `map.rs`, `option.rs` and
//! `bytes.rs`) decide what each one takes and gives back; when one is added
//! there, add it here too.

use crate::facts::MemberKind;
use crate::symbols::SymbolKind;
use crate::types::Type;

/// The built-in fields and methods of a type.
pub fn built_in_members(ty: &Type) -> Vec<(&'static str, MemberKind)> {
    use MemberKind::{Field, Operation};

    let fields = |names: &[&'static str]| names.iter().map(|n| (*n, Field)).collect::<Vec<_>>();
    let methods =
        |names: &[&'static str]| names.iter().map(|n| (*n, Operation)).collect::<Vec<_>>();

    let mut members = match ty {
        Type::String => {
            let mut members = fields(&["length", "is_empty"]);

            members.extend(methods(&[
                "contains",
                "starts_with",
                "ends_with",
                "to_upper",
                "to_lower",
                "trim",
                "slice",
                "split",
                "to_int",
                "to_bytes",
            ]));

            members
        }
        Type::Bytes => {
            let mut members = fields(&["length", "is_empty"]);
            members.extend(methods(&["get", "add", "slice", "to_text"]));
            members
        }
        Type::List(item) => {
            let mut members = fields(&["length", "is_empty", "first", "last"]);

            members.extend(methods(&[
                "contains", "get", "add", "drop", "take", "filter", "all", "any", "count", "find",
                "sum", "min_by", "max_by", "map", "reverse", "sort_by",
            ]));

            if item.named(&[SymbolKind::Entity]).is_some() {
                members.extend(methods(&["remove", "replace"]));
            } else {
                members.push(("index_of", Operation));
            }

            if item.is_number() || **item == Type::String {
                members.push(("sort", Operation));
            }

            members
        }
        Type::Map(..) => {
            let mut members = fields(&["length", "is_empty", "keys", "values"]);
            members.extend(methods(&["get", "set", "remove", "contains"]));
            members
        }
        Type::Option(_) => {
            let mut members = fields(&["is_some", "is_none"]);
            members.extend(methods(&["map", "unwrap_or"]));
            members
        }
        Type::Int | Type::Float => methods(&["to_string"]),
        Type::Decimal(_) => methods(&["to_string", "round"]),
        Type::Named(named) if named.kind == SymbolKind::Event => fields(&["source"]),
        _ => Vec::new(),
    };

    members.dedup();

    members
}
