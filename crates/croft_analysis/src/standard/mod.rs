//! The standard contexts: built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import, and nor do the names
//! of `Std.Storage` and `Std.Console`.

/// Each standard context and the names it provides.
pub const STANDARD_CONTEXTS: [(&str, &[&str]); 3] = [
    (
        "Std.Text",
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
];

/// The names a standard context provides, if `context` is one.
pub fn standard_names(context: &str) -> Option<&'static [&'static str]> {
    STANDARD_CONTEXTS
        .iter()
        .find(|(name, _)| *name == context)
        .map(|(_, names)| *names)
}

/// The standard context that provides `name`, if one does.
pub fn standard_context_of(name: &str) -> Option<&'static str> {
    STANDARD_CONTEXTS
        .iter()
        .find(|(_, names)| names.contains(&name))
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
