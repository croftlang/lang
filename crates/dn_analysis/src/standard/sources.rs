//! Adding the standard contexts written in dn to a program, and naming the
//! import a missing standard name needs.

use dn_syntax::ast::{ContextItem, File};
use dn_syntax::{parse, SourceId};

use super::{standard_source, STANDARD_SOURCES, STORAGE_CONTEXT};

/// The source id given to standard contexts, which no project file has.
pub const STANDARD_SOURCE: SourceId = SourceId(u32::MAX);

/// Adds the standard contexts written in dn that the program needs: those
/// any file imports from, `Std.Storage` when there is a repository, and
/// `Std.Files` when a target saves a repository in `FileStorage`.
pub fn add_standard_sources(files: &mut Vec<File>, saves_in_files: bool) {
    let mut imported: Vec<String> = Vec::new();
    let has_repository = (files
        .iter()
        .flat_map(|f| &f.contexts)
        .flat_map(|c| &c.items))
    .any(|item| matches!(item, ContextItem::Repository(_)));
    if has_repository {
        imported.push(STORAGE_CONTEXT.to_string());
    }
    if saves_in_files {
        imported.push("Std.Files".to_string());
    }
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
