//! C names for Croft declarations and types.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;

use crate::Generator;

/// `Dungeon_Room_Enemy` for `Room.Enemy` in context `Dungeon`, and
/// `Shop_Pair_Int_String` for the instance `Pair<Int, String>`.
pub fn c_name(named: &Resolved) -> String {
    format!(
        "{}_{}",
        c_identifier(&named.context),
        c_identifier(&named.name)
    )
}

/// A Croft name as a C identifier: anything but letters, digits and `_` becomes
/// `_`, and runs of them one, with none at the end.
fn c_identifier(name: &str) -> String {
    let mut out = String::new();

    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }

    out.trim_end_matches('_').to_string()
}

/// The C name for an error's code: `CROFT_ERROR_Dungeon_Room_NotFound`.
pub fn error_code(error: &Resolved) -> String {
    format!("CROFT_ERROR_{}", c_name(error))
}

/// The C name for an event's kind in the event queue.
pub fn event_kind(event: &Resolved) -> String {
    format!("CROFT_EVENT_{}", c_name(event))
}

/// The C name for an aggregate's kind, which its queued events carry.
pub fn aggregate_kind(aggregate: &Resolved) -> String {
    format!("CROFT_AGGREGATE_{}", c_name(aggregate))
}

/// Escapes text for a C string literal.
pub fn c_string(text: &str) -> String {
    let mut escaped = String::from("\"");

    for c in text.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            c if c.is_ascii() && !c.is_ascii_control() => escaped.push(c),
            _ => escaped.push('?'),
        }
    }

    escaped.push('"');

    escaped
}

impl<'a> Generator<'a> {
    /// The C type for a Croft type. Every error type shares `croft_error`. Using a
    /// type here records it, so its definition is generated.
    pub fn c_type(&self, ty: &Type) -> String {
        let ty = self.canonical(ty);

        let name = match &ty {
            Type::Int | Type::Decimal(_) => "croft_int".to_string(),
            Type::Float => "double".to_string(),
            Type::Bool => "croft_bool".to_string(),
            Type::String => "croft_string".to_string(),
            Type::Text(length) => {
                self.texts.borrow_mut().insert(i64::from(*length));
                format!("croft_text_{length}")
            }
            Type::Borrowed(_) => "croft_string_view".to_string(),
            Type::Bytes => "croft_bytes".to_string(),
            Type::Unit | Type::Unknown | Type::Port(_) | Type::Repository(_) => {
                "croft_unit".to_string()
            }
            Type::Errors(_) => "croft_error".to_string(),
            Type::Named(named) => c_name(named),
            Type::List(item) => format!("croft_list_{}", self.short(item)),
            Type::Map(key, value) => format!("croft_map_{}_{}", self.short(key), self.short(value)),
            Type::Option(item) => format!("croft_option_{}", self.short(item)),
            Type::Result(value, error) => {
                format!("croft_result_{}_{}", self.short(value), self.short(error))
            }
        };

        self.use_type(&ty);

        name
    }

    /// One form for each type that shares a C type: all errors become `Errors([])`.
    pub fn canonical(&self, ty: &Type) -> Type {
        match ty {
            Type::Errors(_) => Type::Errors(Vec::new()),
            Type::Named(_) if self.model.is_error(ty) => Type::Errors(Vec::new()),
            Type::List(item) => Type::list(self.canonical(item)),
            Type::Map(key, value) => Type::map(self.canonical(key), self.canonical(value)),
            Type::Option(item) => Type::option(self.canonical(item)),
            Type::Result(value, error) => {
                Type::result(self.canonical(value), self.canonical(error))
            }
            Type::Unknown | Type::Port(_) | Type::Repository(_) => Type::Unit,
            Type::Text(length) if i64::from(*length) >= self.sizes.string_capacity => Type::String,
            other => other.clone(),
        }
    }

    fn short(&self, ty: &Type) -> String {
        let name = self.c_type(ty);

        name.strip_prefix("croft_")
            .map(str::to_string)
            .unwrap_or(name)
    }

    fn use_type(&self, ty: &Type) {
        let mut used = self.used_types.borrow_mut();

        if used.1.insert(ty.clone()) {
            used.0.push(ty.clone());
        }
    }

    /// Passed and stored by value in C: numbers, flags, identities and plain enums.
    pub fn is_scalar(&self, ty: &Type) -> bool {
        match self.canonical(ty) {
            Type::Int | Type::Float | Type::Decimal(_) | Type::Bool | Type::Unit => true,
            Type::Named(named) => match named.kind {
                SymbolKind::Identity => true,
                SymbolKind::Enum => self.is_plain_enum(&named),
                _ => false,
            },
            _ => false,
        }
    }

    /// An enum whose variants carry no data is just a number in C.
    pub fn is_plain_enum(&self, named: &Resolved) -> bool {
        self.model
            .variants(named)
            .iter()
            .all(|variant| variant.fields.is_empty())
    }
}
