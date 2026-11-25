//! What is under the cursor, what to show on hover, and where it is declared.

use b_analysis::symbols::SymbolKind;
use b_analysis::types::{Resolved, Type};
use b_analysis::{Context, LocalName, Member, MemberKind, Reference};
use b_syntax::ast::{Field, Identifier};
use b_syntax::{SourceId, Span};

use super::session::ProjectState;

/// Something with a name under the cursor.
pub enum Found<'a> {
    /// A name that refers to a declaration.
    Reference(&'a Reference),
    /// A parameter, `let` name, binding or loop counter.
    Local(&'a LocalName),
    /// A field, port operation or built-in method after a value, or a field
    /// named in a record.
    Member(&'a Member),
    /// The name in a declaration itself.
    Declaration(Resolved, Span),
    /// The name of a field or operation where it is declared.
    MemberDeclaration(DeclaredMember<'a>),
}

/// A field or operation as declared: `x` in `value Position { x: Int }`.
#[derive(Clone)]
pub struct DeclaredMember<'a> {
    pub name: &'a Identifier,
    /// What it belongs to, as its context names it: `Position`, `Room.Enemy`.
    pub owner: String,
    pub context: &'a str,
    pub kind: MemberKind,
}

/// Every field and operation a context declares.
pub fn declared_members<'a>(context: &'a Context) -> Vec<DeclaredMember<'a>> {
    let mut members = Vec::new();
    let at = context.name.name.as_str();
    let fields = |members: &mut Vec<DeclaredMember<'a>>, owner: &str, fields: &'a [Field]| {
        for field in fields {
            members.push(DeclaredMember {
                name: &field.name,
                owner: owner.to_string(),
                context: at,
                kind: MemberKind::Field,
            });
        }
    };
    for value in &context.values {
        fields(&mut members, &value.name.name, &value.fields);
    }
    for error in &context.errors {
        fields(&mut members, &error.name.name, &error.fields);
    }
    for decl in &context.enums {
        for variant in &decl.variants {
            let owner = format!("{}.{}", decl.name.name, variant.name.name);
            fields(&mut members, &owner, &variant.fields);
        }
    }
    for aggregate in &context.aggregates {
        let name = &aggregate.name.name;
        fields(&mut members, name, &aggregate.fields);
        for entity in &aggregate.entities {
            fields(
                &mut members,
                &format!("{name}.{}", entity.name.name),
                &entity.fields,
            );
        }
        for record in aggregate.errors.iter().chain(&aggregate.events) {
            fields(
                &mut members,
                &format!("{name}.{}", record.name.name),
                &record.fields,
            );
        }
    }
    for adapter in &context.adapters {
        fields(&mut members, &adapter.name.name, &adapter.fields);
    }
    let ports = (context.ports.iter()).map(|p| (&p.name.name, &p.operations));
    let adapters = (context.adapters.iter()).map(|a| (&a.name.name, &a.operations));
    for (owner, operations) in ports.chain(adapters) {
        for operation in operations {
            members.push(DeclaredMember {
                name: &operation.name,
                owner: owner.clone(),
                context: at,
                kind: MemberKind::Operation,
            });
        }
    }
    members
}

impl Found<'_> {
    /// Where the name under the cursor is written, and how long it is.
    pub fn written(&self) -> (Span, usize) {
        match self {
            Found::Reference(reference) => (reference.span, reference.name.chars().count()),
            Found::Local(local) => (local.span, local.name.chars().count()),
            Found::Member(member) => (member.span, member.name.chars().count()),
            Found::Declaration(target, span) => (*span, last_segment(&target.name).chars().count()),
            Found::MemberDeclaration(member) => {
                (member.name.span, member.name.name.chars().count())
            }
        }
    }
}

fn last_segment(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn covers(span: Span, length: usize, id: SourceId, line: u32, column: u32) -> bool {
    span.source == id
        && span.line == line
        && span.column <= column
        && column < span.column + length as u32
}

/// The name written at `line` and `column` of a file.
pub fn find_at(state: &ProjectState, id: SourceId, line: u32, column: u32) -> Option<Found<'_>> {
    let facts = &state.analysis.program.facts;
    let at = |span: Span, name: &str| covers(span, name.chars().count(), id, line, column);
    if let Some(reference) = facts.references.iter().find(|r| at(r.span, &r.name)) {
        return Some(Found::Reference(reference));
    }
    if let Some(member) = facts.members.iter().find(|m| at(m.span, &m.name)) {
        return Some(Found::Member(member));
    }
    if let Some(local) = facts.locals.iter().find(|l| at(l.span, &l.name)) {
        return Some(Found::Local(local));
    }
    for (context, table) in &state.analysis.symbols {
        for (name, symbol) in &table.symbols {
            if at(symbol.span, last_segment(name)) {
                let target = Resolved {
                    context: context.clone(),
                    name: name.clone(),
                    kind: symbol.kind,
                };
                return Some(Found::Declaration(target, symbol.span));
            }
        }
    }
    let contexts = state.analysis.program.contexts.values();
    (contexts.flat_map(declared_members))
        .find(|m| at(m.name.span, &m.name.name))
        .map(Found::MemberDeclaration)
}

/// Where a declaration's name is written. `written` is the name as used:
/// for an enum's variant it is the variant, and the target is the enum.
pub fn declaration_span(state: &ProjectState, target: &Resolved, written: &str) -> Option<Span> {
    if target.kind == SymbolKind::Enum && last_segment(&target.name) != written {
        let context = state.analysis.program.contexts.get(&target.context)?;
        let enum_decl = context
            .enums
            .iter()
            .find(|e| e.name.name == last_segment(&target.name))?;
        let variant = enum_decl.variants.iter().find(|v| v.name.name == written)?;
        return Some(variant.name.span);
    }
    let symbol = state
        .analysis
        .symbols
        .get(&target.context)?
        .get(&target.name)?;
    Some(symbol.span)
}

/// Where the thing under the cursor is declared, and the length of its name.
pub fn definition(state: &ProjectState, found: &Found) -> Option<(Span, usize)> {
    match found {
        Found::Reference(reference) => {
            let span = declaration_span(state, &reference.target, &reference.name)?;
            Some((span, reference.name.chars().count()))
        }
        Found::Local(local) => match (&local.defined, &local.ty) {
            (Some(span), _) => Some((*span, local.name.chars().count())),
            // `screen` in a service that `calls Screen` goes to the port.
            (None, Type::Port(port) | Type::Repository(port)) => {
                let span = declaration_span(state, port, last_segment(&port.name))?;
                Some((span, last_segment(&port.name).chars().count()))
            }
            (None, _) => None,
        },
        Found::Member(member) => Some((member.defined?, member.name.chars().count())),
        Found::Declaration(target, span) => {
            Some((*span, last_segment(&target.name).chars().count()))
        }
        Found::MemberDeclaration(member) => {
            Some((member.name.span, member.name.name.chars().count()))
        }
    }
}

/// Markdown for hovering over the thing under the cursor.
pub fn hover(state: &ProjectState, found: &Found) -> Option<String> {
    let (target, written) = match found {
        Found::Local(local) => {
            let mut text = format!("```b\n{}: {}\n```", local.name, local.ty);
            if let Type::Port(port) | Type::Repository(port) = &local.ty {
                if let Some(port_text) = describe(state, port, last_segment(&port.name)) {
                    text = format!("{text}\n\n---\n\n{port_text}");
                }
            }
            return Some(text);
        }
        Found::Reference(reference) => (&reference.target, reference.name.as_str()),
        Found::Declaration(target, _) => (target, last_segment(&target.name)),
        Found::Member(member) => {
            let of = match &member.receiver {
                Type::Named(owner) | Type::Port(owner) | Type::Repository(owner)
                    if !owner.context.is_empty() =>
                {
                    format!("`{}` in `{}`", owner.name, owner.context)
                }
                receiver => format!("`{receiver}`"),
            };
            let written = Written {
                name: &member.name,
                ty: Some(&member.ty),
                kind: member.kind,
                defined: member.defined,
            };
            return Some(describe_member(
                state,
                written,
                &of,
                built_in(&member.receiver),
            ));
        }
        Found::MemberDeclaration(member) => {
            let of = format!("`{}` in `{}`", member.owner, member.context);
            let written = Written {
                name: &member.name.name,
                ty: None,
                kind: member.kind,
                defined: Some(member.name.span),
            };
            return Some(describe_member(state, written, &of, false));
        }
    };
    describe(state, target, written)
}

/// Is a value of this type one the language provides, whose methods are
/// not operations of a port?
fn built_in(receiver: &Type) -> bool {
    !matches!(receiver, Type::Port(_) | Type::Repository(_))
}

/// A field or operation, as used or as declared.
struct Written<'a> {
    name: &'a str,
    /// Its type, or what calling it gives back, when known.
    ty: Option<&'a Type>,
    kind: MemberKind,
    defined: Option<Span>,
}

/// Markdown for a field or operation: how it is declared, its doc comment,
/// and what it belongs to.
fn describe_member(state: &ProjectState, member: Written, of: &str, method: bool) -> String {
    let declared = member.defined.and_then(|span| {
        let text = state.text(span.source)?;
        let written = match member.kind {
            MemberKind::Field => {
                let line = text.lines().nth(span.line as usize - 1)?;
                strip_comment(line).trim().to_string()
            }
            MemberKind::Operation => declaration_text(text, span.line, SymbolKind::Function, false),
        };
        Some((written, doc_above(text, span.line)))
    });
    let unknown = Type::Unknown;
    let ty = member.ty.unwrap_or(&unknown);
    let (written, doc) = declared.unwrap_or_else(|| match member.kind {
        MemberKind::Field => (format!("{}: {ty}", member.name), String::new()),
        MemberKind::Operation => (format!("fn {}(…): {ty}", member.name), String::new()),
    });
    let what = match member.kind {
        MemberKind::Field => "A field",
        MemberKind::Operation if method => "A method",
        MemberKind::Operation => "An operation",
    };
    let mut markdown = format!("```b\n{written}\n```");
    if !doc.is_empty() {
        markdown.push_str(&format!("\n\n{doc}"));
    }
    markdown.push_str(&format!("\n\n*{what} of {of}*"));
    markdown
}

/// A declaration's first lines, its doc comment, and what kind of thing it is.
fn describe(state: &ProjectState, target: &Resolved, written: &str) -> Option<String> {
    let span = declaration_span(state, target, written)?;
    let text = state.text(span.source)?;
    let is_variant = target.kind == SymbolKind::Enum && last_segment(&target.name) != written;
    let header = match target.kind {
        SymbolKind::Aggregate => aggregate_outline(state, target)?,
        kind => declaration_text(text, span.line, kind, is_variant),
    };
    let doc = doc_above(text, span.line);
    let owner = match target.name.rsplit_once('.') {
        Some((outer, _)) => format!("{}.{outer}", target.context),
        None => target.context.clone(),
    };
    let what = if is_variant {
        format!("A variant of `{}`", last_segment(&target.name))
    } else {
        capitalize(target.kind.describe())
    };
    let mut markdown = format!("```b\n{header}\n```");
    if !doc.is_empty() {
        markdown.push_str(&format!("\n\n{doc}"));
    }
    markdown.push_str(&format!("\n\n*{what} in `{owner}`*"));
    Some(markdown)
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The `//` lines directly above a line, without their slashes.
fn doc_above(text: &str, line: u32) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut doc = Vec::new();
    let mut index = line as usize - 1;
    while index > 0 {
        index -= 1;
        let Some(comment) = lines[index].trim().strip_prefix("//") else {
            break;
        };
        doc.push(comment.trim());
    }
    doc.reverse();
    doc.join("\n")
}

/// How a declaration is written. Small declarations whose body says what
/// they are, such as a value's fields or a port's operations, are shown
/// whole. Others show their signature without the body.
fn declaration_text(text: &str, line: u32, kind: SymbolKind, is_variant: bool) -> String {
    use SymbolKind::*;
    let lines: Vec<&str> = text.lines().skip(line as usize - 1).take(40).collect();
    let whole = !is_variant && matches!(kind, Value | Entity | Error | Event | Enum | Port);
    let mut taken = Vec::new();
    let (mut parens, mut braces) = (0i32, 0i32);
    for line in &lines {
        let code = strip_comment(line);
        for c in code.chars() {
            match c {
                '(' | '[' => parens += 1,
                ')' | ']' => parens -= 1,
                '{' => braces += 1,
                '}' => braces -= 1,
                _ => {}
            }
        }
        // Comments and blank lines are left out of the lines shown.
        if !code.trim().is_empty() {
            taken.push(code.trim_end());
        }
        let open = parens > 0 || (whole && braces > 0);
        if !open {
            break;
        }
    }
    let indent = taken
        .first()
        .map_or(0, |first| first.len() - first.trim_start().len());
    let mut written: Vec<String> = taken
        .iter()
        .map(|line| line.get(indent..).unwrap_or(line.trim_start()).to_string())
        .collect();
    if !whole {
        if let Some(last) = written.last_mut() {
            *last = signature(last, kind);
        }
    }
    written.join("\n")
}

/// An aggregate with what it holds, from every file it is written in: its
/// fields, entities, errors and events, and its commands' signatures.
fn aggregate_outline(state: &ProjectState, target: &Resolved) -> Option<String> {
    let context = state.analysis.program.contexts.get(&target.context)?;
    let aggregate = (context.aggregates.iter()).find(|a| a.name.name == target.name)?;
    let line_at = |name: &Identifier| -> Option<String> {
        let text = state.text(name.span.source)?;
        let line = text.lines().nth(name.span.line as usize - 1)?;
        Some(signature(strip_comment(line).trim(), SymbolKind::Command))
    };
    let mut lines = vec![format!("aggregate {} {{", target.name)];
    let fields = aggregate.fields.iter().map(|f| &f.name);
    let entities = aggregate.entities.iter().map(|e| &e.name);
    let records = (aggregate.errors.iter().chain(&aggregate.events)).map(|r| &r.name);
    for name in fields.chain(entities).chain(records) {
        lines.extend(line_at(name).map(|line| format!("    {line}")));
    }
    for command in &aggregate.commands {
        let Some(text) = state.text(command.name.span.source) else {
            continue;
        };
        let written = declaration_text(text, command.name.span.line, SymbolKind::Command, false);
        lines.extend(written.lines().map(|line| format!("    {line}")));
    }
    lines.push("}".to_string());
    Some(lines.join("\n"))
}

/// A line without its `//` comment, ignoring `//` inside strings.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut previous = ' ';
    for (index, c) in line.char_indices() {
        match c {
            '"' if previous != '\\' => in_string = !in_string,
            '/' if !in_string && previous == '/' => return &line[..index - 1],
            _ => {}
        }
        previous = c;
    }
    line
}

/// A declaration's line without the body that follows: `= { ...`, `{` or,
/// for something with a signature, `= expression`.
fn signature(line: &str, kind: SymbolKind) -> String {
    use SymbolKind::*;
    let mut line = line.trim_end();
    if let Some(open) = line.strip_suffix('{') {
        line = open.trim_end();
        line = line.strip_suffix('=').unwrap_or(line).trim_end();
    }
    let has_body = matches!(
        kind,
        Command | Function | Service | Translation | Constraint | Policy | Adapter
    );
    if has_body {
        let mut depth = 0;
        for (index, c) in line.char_indices() {
            match c {
                '(' | '[' | '{' | '<' => depth += 1,
                ')' | ']' | '}' | '>' => depth -= 1,
                '=' if depth == 0 && line[index..].starts_with("= ") => {
                    return line[..index].trim_end().to_string()
                }
                _ => {}
            }
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_leaves_out_the_body() {
        let line =
            "fn act(room: Room, key: Key): Result<Room, Room.Error> = match direction_for(key) {";
        assert_eq!(
            signature(line, SymbolKind::Function),
            "fn act(room: Room, key: Key): Result<Room, Room.Error>"
        );
        let line = "command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20)) {";
        assert_eq!(
            signature(line, SymbolKind::Command),
            "command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20))"
        );
        assert_eq!(
            signature("adapter A for Screen = {", SymbolKind::Adapter),
            "adapter A for Screen"
        );
        assert_eq!(
            signature("const MAX: Int = 16", SymbolKind::Const),
            "const MAX: Int = 16"
        );
    }

    #[test]
    fn a_port_is_shown_whole_and_without_comments() {
        let text = "// The screen.\nport Screen {\n    // Wipes it.\n    fn clear(): Unit // wipes it\n}\nport Other {}";
        assert_eq!(
            declaration_text(text, 2, SymbolKind::Port, false),
            "port Screen {\n    fn clear(): Unit\n}"
        );
        assert_eq!(doc_above(text, 2), "The screen.");
    }

    #[test]
    fn a_signature_can_span_lines() {
        let text = "service Play(\n    room_id: RoomId,\n): Unit = {\n}";
        assert_eq!(
            declaration_text(text, 1, SymbolKind::Service, false),
            "service Play(\n    room_id: RoomId,\n): Unit"
        );
    }
}
