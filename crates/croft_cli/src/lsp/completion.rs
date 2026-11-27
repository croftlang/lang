//! What to offer as the user types: after a `.`, the fields and operations
//! of what comes before it, or the variants and members of a type; anywhere
//! else, the names in scope, the context's declarations, and keywords.
//!
//! The text being typed often does not parse, so the analysis may be of an
//! earlier version of the file. Names are matched by what they are called,
//! and locals by roughly where they are, so that this still works.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::{Resolved, Type};
use croft_analysis::{built_in_members, MemberKind, Model};
use croft_syntax::ast::{Identifier, Path};
use croft_syntax::keyword::Keyword;
use croft_syntax::{SourceId, Span};
use lsp_types::{CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind};

use super::lookup::{self, declared_members};
use super::session::ProjectState;

/// Words with a meaning in one place only, which the lexer leaves as names.
const CONTEXTUAL_WORDS: [&str; 8] = [
    "from", "as", "on", "for", "in", "when", "include", "contexts",
];

/// The built-in types, offered anywhere.
const BUILT_IN_TYPES: [&str; 12] = [
    "Int", "Float", "Decimal", "Bool", "String", "Bytes", "Unit", "List", "Map", "Set", "Option",
    "Result",
];

/// The built-in functions and constraints, offered anywhere.
const BUILT_IN_FUNCTIONS: [&str; 16] = [
    "Some",
    "Ok",
    "Err",
    "max",
    "min",
    "abs",
    "clamp",
    "pow",
    "sign",
    "checked_add",
    "checked_subtract",
    "checked_multiply",
    "from_code",
    "from_hex",
    "Range",
    "Length",
];

/// What to offer at `line` and `column` of a file whose text is now `text`.
pub fn complete(
    state: &ProjectState,
    id: SourceId,
    text: &str,
    line: u32,
    column: u32,
) -> Vec<CompletionItem> {
    let Some(context) = file_context(state, id) else {
        return keywords();
    };

    let line_text = text.lines().nth(line as usize - 1).unwrap_or("");
    let before: String = line_text.chars().take(column as usize - 1).collect();
    let typed = before.trim_end_matches(is_word_char);
    let model = Model::new(&state.analysis.program);

    let Some(receiver) = typed.strip_suffix('.') else {
        return in_scope(state, &model, context, id, line);
    };

    let chain: Vec<&str> = receiver
        .rsplit(|c: char| !is_word_char(c) && c != '.')
        .next()
        .unwrap_or("")
        .split('.')
        .collect();

    if chain.iter().any(|segment| segment.is_empty()) {
        return Vec::new();
    }

    let at = Span {
        source: id,
        line,
        column,
    };

    match target(state, &model, context, &chain, at) {
        Some(Target::Value(ty)) => members_of(state, &model, &ty),
        Some(Target::Declaration(declaration)) => inside(state, &model, &declaration),
        None => Vec::new(),
    }
}

/// Whether `c` can be part of a name.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a name starts with a capital, as a type's name does.
fn is_type_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// The context a file's declarations are in.
fn file_context(state: &ProjectState, id: SourceId) -> Option<&str> {
    let mut contexts = state.analysis.program.contexts.iter();

    let found = contexts.find(|(name, context)| {
        context.name.span.source == id
            || (state.analysis.symbols.get(*name))
                .is_some_and(|table| table.symbols.values().any(|s| s.span.source == id))
    });

    found.map(|(name, _)| name.as_str())
}

/// What comes before a `.`: a value of some type, or a declaration.
enum Target {
    Value(Type),
    Declaration(Resolved),
}

/// What the names in `chain`, written before a `.` at `at`, refer to.
fn target(
    state: &ProjectState,
    model: &Model,
    context: &str,
    chain: &[&str],
    at: Span,
) -> Option<Target> {
    let facts = &state.analysis.program.facts;
    let first = chain[0];

    // A port or repository in `calls`, such as `Screen`, or its alias, is a
    // value for the whole body, whatever its case.
    let handle = (facts.locals.iter())
        .find(|l| l.name == first && l.defined.is_none() && l.span.source == at.source)
        .map(|local| local.ty.clone());

    let mut found = if let Some(handle) = handle {
        handle
    } else if is_type_name(first) {
        // The longest run of type names that names a declaration, as in `Room.Error`.
        let names = chain.iter().take_while(|name| is_type_name(name)).count();

        let (declaration, used) = (1..=names).rev().find_map(|count| {
            let path = path(&chain[..count]);
            Some((model.resolve(context, None, &path)?, count))
        })?;

        if used < chain.len() {
            // A field of a constant, for one, is not looked into.
            return None;
        }

        return Some(Target::Declaration(declaration));
    } else {
        // The latest place the name was given or used before the cursor.
        let before = |span: Span| {
            span.source == at.source && (span.line, span.column) < (at.line, at.column)
        };

        let local = (facts.locals.iter())
            .filter(|local| local.name == first && before(local.span))
            .max_by_key(|local| (local.span.line, local.span.column))
            .map(|local| local.ty.clone());

        let own_field = || {
            (facts.members.iter())
                .find(|m| {
                    m.name == first && m.kind == MemberKind::Field && m.span.source == at.source
                })
                .map(|m| m.ty.clone())
        };

        local.or_else(own_field)?
    };

    for name in &chain[1..] {
        found = field_type(state, model, &found, name)?;
    }

    Some(Target::Value(found))
}

/// A path made of `names`, with no place in a file.
fn path(names: &[&str]) -> Path {
    let segments = names
        .iter()
        .map(|name| Identifier {
            name: name.to_string(),
            span: Span::default(),
        })
        .collect();

    Path { segments }
}

/// The type of a field of a value of type `ty`.
fn field_type(state: &ProjectState, model: &Model, ty: &Type, name: &str) -> Option<Type> {
    let members = &state.analysis.program.facts.members;

    if let Some(member) = (members.iter())
        .find(|m| m.name == name && m.receiver == *ty && m.kind == MemberKind::Field)
    {
        return Some(member.ty.clone());
    }

    let Type::Named(named) = ty else {
        return None;
    };

    let fields = model.fields(named);
    let (_, ty, _) = fields.into_iter().find(|(field, _, _)| field == name)?;

    Some(ty)
}

/// The fields and operations of a value of type `ty`.
fn members_of(state: &ProjectState, model: &Model, ty: &Type) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    match ty {
        Type::Named(named) if state.analysis.program.contexts.contains_key(&named.context) => {
            for (name, field_type, field) in model.fields(named) {
                let doc = field.doc.join("\n");

                items.push(item(
                    &name,
                    CompletionItemKind::FIELD,
                    &field_type.to_string(),
                    &doc,
                    1,
                ));
            }
        }
        Type::Port(port) | Type::Repository(port) if !port.context.is_empty() => {
            for operation in model.operations_of(port) {
                let parameters: Vec<String> = (operation.parameters.iter())
                    .map(|(name, ty, _)| format!("{name}: {ty}"))
                    .collect();

                let signature = format!(
                    "fn {}({}): {}",
                    operation.name,
                    parameters.join(", "),
                    operation.result
                );

                let doc = port_operation_doc(state, port, &operation.name);

                items.push(item(
                    &operation.name,
                    CompletionItemKind::METHOD,
                    &signature,
                    &doc,
                    1,
                ));
            }
        }
        _ => {}
    }

    for (name, kind) in built_in_members(ty) {
        let kind = match kind {
            MemberKind::Field => CompletionItemKind::FIELD,
            MemberKind::Operation => CompletionItemKind::METHOD,
        };

        if !items.iter().any(|item| item.label == name) {
            items.push(item(name, kind, &format!("built into `{ty}`"), "", 2));
        }
    }

    items
}

/// The doc comment on a port's operation.
fn port_operation_doc(state: &ProjectState, port: &Resolved, name: &str) -> String {
    let Some(context) = state.analysis.program.contexts.get(&port.context) else {
        return String::new();
    };

    let operation = (context.ports.iter())
        .filter(|p| p.name.name == port.name)
        .flat_map(|p| &p.operations)
        .find(|operation| operation.name.name == name);

    operation.map_or(String::new(), |operation| operation.doc.join("\n"))
}

/// What can follow a declaration's name and a `.`: an enum's variants, or
/// an aggregate's entities, errors, events and commands.
fn inside(state: &ProjectState, model: &Model, declaration: &Resolved) -> Vec<CompletionItem> {
    use SymbolKind::*;

    match declaration.kind {
        Enum | ErrorSet => model
            .variants(declaration)
            .into_iter()
            .map(|variant| {
                let fields: Vec<String> = (variant.fields.iter())
                    .map(|(name, ty)| format!("{name}: {ty}"))
                    .collect();

                let detail = match fields.is_empty() {
                    true => format!("{}.{}", declaration.name, variant.name),
                    false => format!("{} {{ {} }}", variant.name, fields.join(", ")),
                };

                item(
                    &variant.name,
                    CompletionItemKind::ENUM_MEMBER,
                    &detail,
                    "",
                    1,
                )
            })
            .collect(),
        Aggregate => {
            let Some(table) = state.analysis.symbols.get(&declaration.context) else {
                return Vec::new();
            };

            let prefix = format!("{}.", declaration.name);
            let mut items = Vec::new();

            for (name, symbol) in &table.symbols {
                let Some(member) = name.strip_prefix(&prefix) else {
                    continue;
                };

                let target = Resolved {
                    context: declaration.context.clone(),
                    name: name.clone(),
                    kind: symbol.kind,
                };

                items.push(declaration_item(state, member, &target, 1));
            }

            items
        }
        _ => Vec::new(),
    }
}

/// Everything that can be written at a place with no `.` before it.
fn in_scope(
    state: &ProjectState,
    model: &Model,
    context: &str,
    id: SourceId,
    line: u32,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();
    let table = state.analysis.symbols.get(context);

    // The innermost declaration the cursor is in: the last to start above it.
    let enclosing = table.and_then(|table| {
        (table.symbols.iter())
            .filter(|(_, s)| s.span.source == id && s.span.line <= line)
            .max_by_key(|(_, s)| s.span.line)
            .map(|(name, symbol)| (name.clone(), symbol.span.line))
    });

    let (owner, start) = match &enclosing {
        Some((name, start)) => (name.split('.').next(), *start),
        None => (None, 1),
    };

    // Where the next declaration starts, which ends this one.
    let end = table.and_then(|table| {
        (table.symbols.values())
            .filter(|s| s.span.source == id && s.span.line > line)
            .map(|s| s.span.line)
            .min()
    });

    let facts = &state.analysis.program.facts;

    for local in &facts.locals {
        let span = local.span;

        // A port in `calls`, such as `Screen`, may first be used further down.
        let last = match local.defined {
            Some(_) => line,
            None => end.unwrap_or(u32::MAX),
        };

        let near = span.source == id && start <= span.line && span.line <= last;

        if near && !items.iter().any(|item| item.label == local.name) {
            items.push(item(
                &local.name,
                CompletionItemKind::VARIABLE,
                &local.ty.to_string(),
                "",
                0,
            ));
        }
    }

    // A command's aggregate's fields, or an adapter's.
    if let Some(owner) = owner {
        let contexts = state.analysis.program.contexts.get(context);

        for member in contexts.into_iter().flat_map(declared_members) {
            if member.owner == owner && member.kind == MemberKind::Field {
                let name = member.name.name.clone();
                let found = lookup::Found::MemberDeclaration(member);
                let doc = lookup::hover(state, &found).unwrap_or_default();
                let name = &name;

                items.push(item(
                    name,
                    CompletionItemKind::FIELD,
                    &format!("a field of `{owner}`"),
                    &doc,
                    1,
                ));
            }
        }
    }

    if let Some(table) = table {
        for (name, symbol) in &table.symbols {
            let written = match (name.split_once('.'), owner) {
                (None, _) => name.as_str(),
                // An aggregate's own members, inside it.
                (Some((aggregate, member)), Some(owner)) if aggregate == owner => member,
                _ => continue,
            };

            if written == "Error" {
                continue;
            }

            let target = Resolved {
                context: context.to_string(),
                name: name.clone(),
                kind: symbol.kind,
            };

            items.push(declaration_item(state, written, &target, 2));
        }
    }

    // Names brought in with `use`.
    if let Some(declared) = state.analysis.program.contexts.get(context) {
        for name in declared.uses.iter().flat_map(|u| &u.names) {
            let written = match &name.alias {
                Some(alias) => Path {
                    segments: vec![alias.clone()],
                },
                None => name.path.clone(),
            };

            let Some(target) = model.resolve(context, None, &written) else {
                continue;
            };

            let written = written.last().name.as_str();

            if !items.iter().any(|item| item.label == written) {
                items.push(declaration_item(state, written, &target, 2));
            }
        }
    }

    for name in BUILT_IN_TYPES {
        items.push(item(name, CompletionItemKind::CLASS, "built in", "", 3));
    }

    for name in BUILT_IN_FUNCTIONS {
        items.push(item(name, CompletionItemKind::FUNCTION, "built in", "", 3));
    }

    items.push(item(
        "None",
        CompletionItemKind::CONSTANT,
        "built in",
        "",
        3,
    ));

    items.extend(keywords());

    items
}

/// Every keyword and contextual word.
fn keywords() -> Vec<CompletionItem> {
    (Keyword::words().chain(CONTEXTUAL_WORDS))
        .map(|word| item(word, CompletionItemKind::KEYWORD, "keyword", "", 4))
        .collect()
}

/// An item for a declaration, with its hover text as documentation.
fn declaration_item(
    state: &ProjectState,
    written: &str,
    target: &Resolved,
    rank: u8,
) -> CompletionItem {
    let found = lookup::Found::Declaration(target.clone(), Span::default());
    let doc = lookup::hover(state, &found).unwrap_or_default();
    let kind = completion_kind(target.kind);
    item(written, kind, target.kind.describe(), &doc, rank)
}

/// The kind of item an editor shows for a kind of declaration.
fn completion_kind(kind: SymbolKind) -> CompletionItemKind {
    use SymbolKind::*;

    match kind {
        Value | Entity | Identity | Error => CompletionItemKind::STRUCT,
        Aggregate | Adapter => CompletionItemKind::CLASS,
        Enum | ErrorSet => CompletionItemKind::ENUM,
        Event => CompletionItemKind::EVENT,
        Command | Factory | Function | Service | Translation | Policy | Constraint => {
            CompletionItemKind::FUNCTION
        }
        Const => CompletionItemKind::CONSTANT,
        Port => CompletionItemKind::INTERFACE,
        Repository => CompletionItemKind::MODULE,
    }
}

/// An item to offer. Lower ranks are listed first: locals, then members,
/// declarations, what is built in, and keywords.
fn item(
    label: &str,
    kind: CompletionItemKind,
    detail: &str,
    doc: &str,
    rank: u8,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: (!detail.is_empty()).then(|| detail.to_string()),
        documentation: (!doc.is_empty()).then(|| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc.to_string(),
            })
        }),
        sort_text: Some(format!("{rank}{label}")),
        ..CompletionItem::default()
    }
}
