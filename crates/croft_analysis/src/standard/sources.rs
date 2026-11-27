//! Adding the standard contexts written in Croft to a program, and naming the
//! import a missing standard name needs.

use croft_syntax::ast::{ContextItem, File};
use croft_syntax::{parse, SourceId};

use super::{standard_source, CONSOLE_CONTEXT, STORAGE_CONTEXT};

/// The source id of a standard context: counting down from the top, so no
/// project file has one, and each context's expressions are told apart.
pub fn standard_source_id(context: &str) -> SourceId {
    let index = croft_std::CONTEXTS
        .iter()
        .position(|standard| standard.name == context);

    SourceId(u32::MAX - index.unwrap_or(0) as u32)
}

/// Adds the standard contexts written in Croft that the program needs:
/// `Std.Console` always, those any file imports or calls a port from,
/// `Std.Storage` when there is a repository, and the context of any standard
/// adapter croft.toml binds, such as `Std.Files` for `StdioFiles`, unless
/// the program declares an adapter of that name itself. `FileStorage` saves
/// through `Files`, so it brings both.
pub fn add_standard_sources(files: &mut Vec<File>, bound: &[String]) {
    let mut imported: Vec<String> = vec![CONSOLE_CONTEXT.to_string()];

    let has_repository = (files
        .iter()
        .flat_map(|f| &f.contexts)
        .flat_map(|c| &c.items))
    .any(|item| matches!(item, ContextItem::Repository(_)));

    if has_repository {
        imported.push(STORAGE_CONTEXT.to_string());
    }

    let declared: Vec<&str> = (files.iter())
        .flat_map(|f| &f.contexts)
        .flat_map(|c| &c.items)
        .filter_map(|item| match item {
            ContextItem::Adapter(adapter) => Some(adapter.name.name.as_str()),
            _ => None,
        })
        .collect();

    for adapter in bound {
        if adapter == super::FILE_STORAGE {
            imported.push("Std.Files".to_string());
        }

        if declared.contains(&adapter.as_str()) {
            continue;
        }

        if let Some(context) = standard_context_publishing(adapter) {
            imported.push(context.to_string());
        }
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
        let uses = context_uses
            .chain(domain_uses)
            .map(|use_decl| &use_decl.from);

        let context_calls = (file.contexts.iter())
            .flat_map(|c| &c.items)
            .filter_map(|item| match item {
                ContextItem::Function(function) => Some(&function.calls),
                _ => None,
            })
            .flatten();

        let domain_calls = file.domains.iter().flat_map(|d| &d.calls);
        let calls = (context_calls.chain(domain_calls)).filter_map(|called| called.from.as_ref());

        for from in uses.chain(calls) {
            let from = from.text();

            if standard_source(&from).is_some() && !imported.contains(&from) {
                imported.push(from);
            }
        }
    }

    imported.sort();
    imported.dedup();

    for name in imported {
        let source = standard_source(&name).unwrap_or_default();

        let Ok(mut file) = parse(source, standard_source_id(&name)) else {
            continue;
        };

        for context in &mut file.contexts {
            context.name.name = name.clone();
        }

        files.push(file);
    }
}

/// The standard context written in Croft that publishes `name`, if one does,
/// so a missing import can be named in the error.
pub fn standard_context_publishing(name: &str) -> Option<&'static str> {
    croft_std::CONTEXTS.iter().find_map(|standard| {
        let context = standard.name;
        let file = parse(standard.source, standard_source_id(context)).ok()?;

        let publishes = file.contexts.iter().flat_map(|c| &c.items).any(|item| {
            matches!(item, ContextItem::Publish(paths) if paths.iter().any(|p| p.text() == name))
        });

        publishes.then_some(context)
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
