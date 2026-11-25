//! Built-in ports every target supplies, and the standard contexts:
//! built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import.

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

/// Ports every target supplies, so a service can call them with no adapter
/// named in `dn.toml`: `Arguments` reads the command line and `Console`
/// prints a line. A port the program declares with the same name replaces it.
pub const BUILTIN_PORTS: [&str; 2] = ["Arguments", "Console"];

pub fn is_builtin_port(name: &str) -> bool {
    BUILTIN_PORTS.contains(&name)
}

/// Standard contexts written in dn, each with the name it is imported by.
/// They join the program only when something imports from them.
pub const STANDARD_SOURCES: [(&str, &str); 3] = [
    ("Std.Files", include_str!("files.dn")),
    ("Std.Http", include_str!("http.dn")),
    (STORAGE_CONTEXT, include_str!("storage.dn")),
];

/// The standard context every program with a repository has, whose names
/// need no import: `StorageError` and its errors.
pub const STORAGE_CONTEXT: &str = "Std.Storage";

/// The built-in adapters for `Storage`: where a repository keeps its
/// aggregates when dn.toml names nothing else, and in files.
pub const MEMORY_STORAGE: &str = "MemoryStorage";
pub const FILE_STORAGE: &str = "FileStorage";

/// The source of a standard context written in dn, if `context` is one.
pub fn standard_source(context: &str) -> Option<&'static str> {
    STANDARD_SOURCES
        .iter()
        .find(|(name, _)| *name == context)
        .map(|(_, source)| *source)
}

mod sources;

pub use sources::{add_standard_sources, not_found_message, standard_source_id};
