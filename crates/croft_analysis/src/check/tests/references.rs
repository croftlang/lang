//! What `analyze` keeps for editors: where each name points, and local names.

use croft_syntax::{parse, SourceId, Span};

use crate::check::analyze;
use crate::check::Analysis;
use crate::symbols::SymbolKind;

fn analyze_source(source: &str) -> Analysis {
    let file = parse(source, SourceId(0)).expect("valid syntax");
    analyze(vec![file], None)
}

/// The reference written at `line` and `column`, as `context.name`.
fn target_at(analysis: &Analysis, line: u32, column: u32) -> Option<(String, SymbolKind)> {
    let span = Span {
        source: SourceId(0),
        line,
        column,
    };
    let facts = &analysis.program.facts;
    let reference = facts.references.iter().find(|r| r.span == span)?;
    let target = &reference.target;
    Some((format!("{}.{}", target.context, target.name), target.kind))
}

const ROOM: &str = "\
context Dungeon {
    identity RoomId for Room: Int where Range(1, 20)
    aggregate Room {
        id: RoomId
        lives: Int
        error NotFound
        command Heal(amount: Int) = {
            let more = lives + amount
            set lives = more
        }
    }
    fn first(): Room.Error = NotFound
}";

#[test]
fn notes_types_and_their_paths() {
    let analysis = analyze_source(ROOM);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    // `RoomId` in `id: RoomId`.
    let (name, kind) = target_at(&analysis, 4, 13).expect("RoomId is noted");
    assert_eq!(name, "Dungeon.RoomId");
    assert_eq!(kind, SymbolKind::Identity);
    // `Room` in `Room.Error` names the aggregate.
    let (name, _) = target_at(&analysis, 12, 17).expect("Room is noted");
    assert_eq!(name, "Dungeon.Room");
}

#[test]
fn notes_names_in_bodies() {
    let analysis = analyze_source(ROOM);
    // `NotFound` in the body of `first`.
    let (name, _) = target_at(&analysis, 12, 30).expect("NotFound is noted");
    assert_eq!(name, "Dungeon.Room.NotFound");
}

#[test]
fn notes_where_locals_are_bound_and_used() {
    let analysis = analyze_source(ROOM);
    let locals = &analysis.program.facts.locals;
    let uses: Vec<_> = locals.iter().filter(|l| l.name == "more").collect();
    assert_eq!(uses.len(), 2, "bound once and used once: {uses:?}");
    let bound = Span {
        source: SourceId(0),
        line: 8,
        column: 17,
    };
    assert!(uses.iter().all(|l| l.defined == Some(bound)));
    assert_eq!(uses[0].ty.to_string(), "Int");
}

#[test]
fn keeps_what_it_learned_when_there_are_errors() {
    let analysis = analyze_source(
        "context Dungeon {\n    value Position { x: Missing }\n    value Size { w: Int }\n}",
    );
    assert_eq!(analysis.diagnostics.len(), 1);
    assert!(analysis.symbols["Dungeon"].get("Size").is_some());
}

#[test]
fn notes_fields_and_operations() {
    let source = "\
context Dungeon {
    value Position {
        x: Int
        y: Int
    }
    port Screen {
        fn draw(at: Position): Unit
    }
    fn right(at: Position): Position = Position { x: at.x + 1, y: at.y }
    service Show(at: Position): Unit {
        calls Screen
    } = {
        screen.draw(at)
    }
}";
    let analysis = analyze_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let members = &analysis.program.facts.members;
    let at = |line: u32, column: u32| {
        (members.iter())
            .find(|m| (m.span.line, m.span.column) == (line, column))
            .unwrap_or_else(|| panic!("a member at {line}:{column} in {members:?}"))
    };
    // `x:` in the record, and `.x` after `at`, are the field on line 3.
    for column in [51, 57] {
        let member = at(9, column);
        assert_eq!(member.name, "x");
        assert_eq!(member.defined.map(|s| s.line), Some(3));
        assert_eq!(member.ty, crate::types::Type::Int);
    }
    // `draw` in `screen.draw(at)` is the operation on line 7.
    let draw = at(13, 16);
    assert_eq!(draw.kind, crate::facts::MemberKind::Operation);
    assert_eq!(draw.defined.map(|s| s.line), Some(7));
}
