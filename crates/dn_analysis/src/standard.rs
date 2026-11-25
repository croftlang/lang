//! Built-in ports every target supplies, and the standard contexts:
//! built-in names a context imports before using,
//! such as `use { trim, split } from Std.Text`. The core types, list and
//! option methods, `Range` and `Length` need no import.

use dn_syntax::ast::{ContextItem, File};
use dn_syntax::{parse, SourceId};

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

/// Standard contexts written in dn, each with the name it is imported by.
/// They join the program only when something imports from them.
pub const STANDARD_SOURCES: [(&str, &str); 1] = [("Std.Files", include_str!("standard/files.dn"))];

/// The source of a standard context written in dn, if `context` is one.
pub fn standard_source(context: &str) -> Option<&'static str> {
    STANDARD_SOURCES
        .iter()
        .find(|(name, _)| *name == context)
        .map(|(_, source)| *source)
}

/// The source id given to standard contexts, which no project file has.
pub const STANDARD_SOURCE: SourceId = SourceId(u32::MAX);

/// Adds the standard contexts written in dn that any file imports from.
pub fn add_standard_sources(files: &mut Vec<File>) {
    let mut imported: Vec<String> = Vec::new();
    for file in files.iter() {
        let context_uses =
            file.contexts
                .iter()
                .flat_map(|c| &c.items)
                .filter_map(|item| match item {
                    ContextItem::Use(use_decl) => Some(use_decl),
                    _ => None,
                });
        let domain_uses = file.domains.iter().flat_map(|d| &d.uses);
        for use_decl in context_uses.chain(domain_uses) {
            let from = use_decl.from.text();
            if standard_source(&from).is_some() && !imported.contains(&from) {
                imported.push(from);
            }
        }
    }
    for name in imported {
        let source = standard_source(&name).unwrap_or_default();
        let Ok(mut file) = parse(source, STANDARD_SOURCE) else {
            continue;
        };
        for context in &mut file.contexts {
            context.name.name = name.clone();
        }
        files.push(file);
    }
}

/// The standard context written in dn that publishes `name`, if one does,
/// so a missing import can be named in the error.
pub fn standard_context_publishing(name: &str) -> Option<&'static str> {
    STANDARD_SOURCES.iter().find_map(|(context, source)| {
        let file = parse(source, STANDARD_SOURCE).ok()?;
        let publishes = file.contexts.iter().flat_map(|c| &c.items).any(|item| {
            matches!(item, ContextItem::Publish(paths) if paths.iter().any(|p| p.text() == name))
        });
        publishes.then_some(*context)
    })
}

/// "cannot find ..." for a name that wasn't found, or the import it needs
/// when a standard context publishes it.
pub fn not_found_message(expected: &str, name: &str) -> String {
    match standard_context_publishing(name) {
        Some(context) => format!("`{name}` needs `use {{ {name} }} from {context}`"),
        None => format!("cannot find {expected} named `{name}`"),
    }
}
