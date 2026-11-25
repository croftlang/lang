//! Built-in ports every target supplies, and the standard contexts:
//! built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import.

/// Each standard context and the names it provides.
pub const STANDARD_CONTEXTS: [(&str, &[&str]); 2] = [
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
