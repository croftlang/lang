//! The standard contexts: built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import, and nor do the names
//! of `Std.Storage` and `Std.Console`.

/// Each standard context and the names it provides.
pub const STANDARD_CONTEXTS: [(&str, &[&str]); 3] = [
    (
        "Std.Text",
        &[
            "contains",
            "ends_with",
            "slice",
            "split",
            "starts_with",
            "to_int",
            "to_bytes",
            "to_text",
            "to_lower",
            "to_upper",
            "trim",
        ],
    ),
    ("Std.Numbers", &["Decimal", "abs", "max", "min"]),
    ("Std.Collections", &["Map"]),
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
/// the `Console` and `Arguments` ports, which every target supplies.
pub const CONSOLE_CONTEXT: &str = "Std.Console";

/// Standard contexts written in Croft, each with the name it is imported by.
/// They join the program only when something imports from them.
pub const STANDARD_SOURCES: [(&str, &str); 4] = [
    ("Std.Files", include_str!("files.croft")),
    ("Std.Http", include_str!("http.croft")),
    (STORAGE_CONTEXT, include_str!("storage.croft")),
    (CONSOLE_CONTEXT, include_str!("console.croft")),
];

/// The standard context every program with a repository has, whose names
/// need no import: `StorageError` and its errors.
pub const STORAGE_CONTEXT: &str = "Std.Storage";

/// The built-in adapters for `Storage`: where a repository keeps its
/// aggregates when croft.toml names nothing else, and in files.
pub const MEMORY_STORAGE: &str = "MemoryStorage";
pub const FILE_STORAGE: &str = "FileStorage";

/// The source of a standard context written in Croft, if `context` is one.
pub fn standard_source(context: &str) -> Option<&'static str> {
    STANDARD_SOURCES
        .iter()
        .find(|(name, _)| *name == context)
        .map(|(_, source)| *source)
}

mod sources;

pub use sources::{add_standard_sources, not_found_message, standard_source_id};
