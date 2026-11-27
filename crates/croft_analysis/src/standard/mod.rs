//! The standard contexts: built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import, and nor do the names
//! of `Std.Storage` and `Std.Console`.

/// Each standard context and the names it provides.
pub const STANDARD_CONTEXTS: [(&str, &[&str]); 4] = [
    (
        TEXT_CONTEXT,
        &[
            "code_at",
            "contains",
            "ends_with",
            "from_code",
            "from_hex",
            "index_of",
            "join",
            "lines",
            "pad_left",
            "pad_right",
            "repeat",
            "replace",
            "slice",
            "split",
            "starts_with",
            "to_decimal",
            "to_hex",
            "to_int",
            "to_bytes",
            "to_text",
            "to_lower",
            "to_upper",
            "trim",
        ],
    ),
    (
        "Std.Numbers",
        &[
            "Decimal",
            "abs",
            "checked_add",
            "checked_multiply",
            "checked_subtract",
            "clamp",
            "max",
            "min",
            "pow",
            "sign",
        ],
    ),
    ("Std.Collections", &["Map", "Set"]),
    (
        ENCODING_CONTEXT,
        &[
            "crc16",
            "crc32",
            "from_base64",
            "from_hex",
            "percent_decode",
            "percent_encode",
            "to_base64",
            "to_hex",
        ],
    ),
];

/// The standard context of hex, base64, percent-encoding and checksums.
pub const ENCODING_CONTEXT: &str = "Std.Encoding";

/// The standard context of strings and character codes.
pub const TEXT_CONTEXT: &str = "Std.Text";

/// The functions two standard contexts both provide, such as `from_hex`,
/// which reads an `Int` in `Std.Text` and `Bytes` in `Std.Encoding`. A
/// method may share a name, since the receiver's type picks one, but a
/// function has nothing to pick by, so a context imports each of these
/// from one context only, and that import decides what it means there.
pub const SHARED_FUNCTIONS: [&str; 1] = ["from_hex"];

/// The names a standard context provides, if `context` is one.
pub fn standard_names(context: &str) -> Option<&'static [&'static str]> {
    STANDARD_CONTEXTS
        .iter()
        .find(|(name, _)| *name == context)
        .map(|(_, names)| *names)
}

/// The standard context that provides `name`, if one does: the first, when
/// more than one does.
pub fn standard_context_of(name: &str) -> Option<&'static str> {
    standard_contexts_of(name).next()
}

/// Every standard context that provides `name`. Two can, for methods of
/// different types, such as `to_hex` on `Bytes` from `Std.Encoding` and on
/// `Int` from `Std.Text`.
pub fn standard_contexts_of(name: &str) -> impl Iterator<Item = &'static str> + '_ {
    STANDARD_CONTEXTS
        .iter()
        .filter(move |(_, names)| names.contains(&name))
        .map(|(context, _)| *context)
}

/// The standard context every program has, whose names need no import:
/// the `Console`, `Arguments`, `Environment` and `Process` ports, which
/// every target supplies.
pub const CONSOLE_CONTEXT: &str = "Std.Console";

/// The standard context every program with a repository has, whose names
/// need no import: `StorageError` and its errors.
pub const STORAGE_CONTEXT: &str = "Std.Storage";

/// The built-in adapter for `Storage` that keeps aggregates in memory: where
/// a repository keeps them when croft.toml names nothing else.
pub const MEMORY_STORAGE: &str = "MemoryStorage";
/// The built-in adapter for `Storage` that keeps aggregates in files.
pub const FILE_STORAGE: &str = "FileStorage";

/// The source of a standard context written in Croft, if `context` is one.
/// The sources are kept in `croft_std`, beside the C of their adapters; they
/// join the program only when something imports from them.
pub fn standard_source(context: &str) -> Option<&'static str> {
    croft_std::context(context).map(|context| context.source)
}

mod sources;

pub use sources::{
    add_standard_sources, not_found_message, standard_context_publishing, standard_source_id,
};
