//! The server's answers for the dungeon example, as an editor would ask.

use std::path::{Path, PathBuf};

use super::lookup::{self, Found};
use super::session::{ProjectState, Session};

fn dungeon() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon")
}

/// Checks the dungeon with `file` changed to `text` when given.
fn checked(file: &str, text: Option<&str>) -> (Session, PathBuf) {
    let root = dungeon().canonicalize().expect("the example exists");
    let path = root.join(file);
    let mut session = Session::new(Some(root));
    if let Some(text) = text {
        session.open(path.clone(), text.to_string());
    }
    session.refresh(&path).expect("the project loads");
    (session, path)
}

/// What is under the `occurrence`th `word` on the first line containing `line`.
fn found_at<'a>(state: &'a ProjectState, path: &Path, line: &str, word: &str) -> Option<Found<'a>> {
    let file = state.file_at(path)?;
    let (index, text) = file
        .text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(line))?;
    let column = text.find(word)? as u32 + 2;
    lookup::find_at(state, file.id, index as u32 + 1, column)
}

#[test]
fn hover_shows_a_commands_signature_and_doc() {
    let (session, path) = checked("dungeon/play.b", None);
    let state = session.project_for(&path).expect("checked");
    let found = found_at(state, &path, "Some(direction) ->", "Move").expect("Move is found");
    let hover = lookup::hover(state, &found).expect("a hover");
    assert!(
        hover.contains("command Move(direction: Direction)"),
        "{hover}"
    );
    assert!(hover.contains("Moves the player one step"), "{hover}");
    let (span, _) = lookup::definition(state, &found).expect("a definition");
    let file = state.file(span.source).expect("a project file");
    assert!(file.path.ends_with("dungeon/room/commands.b"));
}

#[test]
fn a_port_handle_goes_to_its_port() {
    let (session, path) = checked("dungeon/play.b", None);
    let state = session.project_for(&path).expect("checked");
    let found = found_at(state, &path, "screen.draw_sprite", "screen").expect("screen is found");
    let (span, _) = lookup::definition(state, &found).expect("a definition");
    let file = state.file(span.source).expect("a project file");
    assert!(file.path.ends_with("dungeon/ports.b"));
}

#[test]
fn a_file_that_stops_parsing_keeps_its_last_tree() {
    let (mut session, path) = checked("dungeon/play.b", None);
    let broken = std::fs::read_to_string(&path).unwrap() + "\nfn broken(";
    session.open(path.clone(), broken);
    session.refresh(&path).expect("the project loads");
    let state = session.project_for(&path).expect("checked");
    assert_eq!(state.diagnostics.len(), 1, "only the syntax error");
    let found = found_at(state, &path, "Some(key) ->", "act");
    assert!(found.is_some(), "act is still found");
}

#[test]
fn errors_point_at_the_file_they_are_in() {
    let original = std::fs::read_to_string(dungeon().join("dungeon/play.b")).unwrap();
    let text = original.replace("rooms.save(changed)?", "rooms.save(chnged)?");
    let (session, path) = checked("dungeon/play.b", Some(&text));
    let state = session.project_for(&path).expect("checked");
    let file = state.file_at(&path).expect("play.b is checked");
    let messages: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| d.span.source == file.id)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(messages, ["cannot find `chnged` here"]);
}

fn hover_at(state: &ProjectState, path: &Path, line: &str, word: &str) -> String {
    let found = found_at(state, path, line, word).unwrap_or_else(|| panic!("{word} is found"));
    lookup::hover(state, &found).expect("a hover")
}

fn definition_file(state: &ProjectState, path: &Path, line: &str, word: &str) -> PathBuf {
    let found = found_at(state, path, line, word).unwrap_or_else(|| panic!("{word} is found"));
    let (span, _) = lookup::definition(state, &found).expect("a definition");
    match state.file(span.source) {
        Some(file) => file.path.clone(),
        None => super::standard::path_of(span.source).expect("a standard file"),
    }
}

#[test]
fn fields_and_operations_after_a_dot() {
    let (session, path) = checked("dungeon/play.b", None);
    let state = session.project_for(&path).expect("checked");
    let line = "screen.draw_sprite(PLAYER_SPRITE, changed.player.position)";
    let hover = hover_at(state, &path, line, "player");
    assert!(hover.contains("player: Player"), "{hover}");
    assert!(
        hover.contains("*A field of `Room` in `Dungeon`*"),
        "{hover}"
    );
    let file = definition_file(state, &path, line, "player");
    assert!(file.ends_with("dungeon/room/room.b"), "{file:?}");
    let hover = hover_at(state, &path, line, "draw_sprite");
    assert!(hover.contains("fn draw_sprite("), "{hover}");
    assert!(hover.contains("*An operation of `Screen`"), "{hover}");
    let file = definition_file(state, &path, line, "draw_sprite");
    assert!(file.ends_with("dungeon/ports.b"), "{file:?}");
    // `find` is the repository's own, with nowhere to go.
    let hover = hover_at(state, &path, "rooms.find(room_id)", "find");
    assert!(hover.contains("fn find(…): Result<Room,"), "{hover}");
}

#[test]
fn an_aggregate_is_shown_with_what_it_holds() {
    let (session, path) = checked("dungeon/play.b", None);
    let state = session.project_for(&path).expect("checked");
    let hover = hover_at(state, &path, "fn act(room: Room", "Room");
    assert!(hover.starts_with("```b\naggregate Room {\n"), "{hover}");
    assert!(hover.contains("\n    player: Player\n"), "{hover}");
    // Commands are written in another file, and shown without their bodies.
    assert!(
        hover.contains("\n    command Move(direction: Direction)\n"),
        "{hover}"
    );
    assert!(!hover.contains("set player"), "{hover}");
}

#[test]
fn a_field_where_it_is_declared() {
    let path = dungeon()
        .canonicalize()
        .unwrap()
        .join("dungeon/room/room.b");
    let (session, _) = checked("dungeon/room/room.b", None);
    let state = session.project_for(&path).expect("checked");
    let hover = hover_at(state, &path, "player: Player", "player");
    assert!(
        hover.contains("*A field of `Room` in `Dungeon`*"),
        "{hover}"
    );
}

#[test]
fn a_standard_declaration_opens_a_file_of_its_own() {
    let (session, path) = checked("platform/spectrum.b", None);
    let state = session.project_for(&path).expect("checked");
    let file = definition_file(state, &path, "use { Files } from Std.Files", "Files");
    assert!(file.ends_with("Std.Files.b"), "{file:?}");
    let text = std::fs::read_to_string(&file).expect("the file is written");
    assert!(text.contains("port Files"));
    assert!(super::standard::source_at(&file).is_some());
}

/// What is offered at the end of `typed`, written on a new line after `after`.
fn completions(after: &str, typed: &str) -> Vec<String> {
    let original = std::fs::read_to_string(dungeon().join("dungeon/play.b")).unwrap();
    let text = original.replacen(after, &format!("{after}\n    {typed}"), 1);
    // The file parsed when it was opened, before the typing began.
    let (mut session, path) = checked("dungeon/play.b", None);
    session.open(path.clone(), text.clone());
    session.refresh(&path).expect("the project loads");
    let state = session.project_for(&path).expect("checked");
    let file = state.file_at(&path).expect("play.b is checked");
    let index = text
        .lines()
        .position(|l| l == format!("    {typed}"))
        .unwrap();
    let column = typed.chars().count() as u32 + 5;
    super::completion::complete(state, file.id, &text, index as u32 + 1, column)
        .into_iter()
        .map(|item| item.label)
        .collect()
}

#[test]
fn completes_what_comes_after_a_dot() {
    let after = "let room = rooms.find(room_id)?";
    let offered = completions(after, "room.");
    for name in ["player", "enemies"] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }
    let offered = completions(after, "room.player.po");
    assert!(offered.iter().any(|o| o == "position"), "{offered:?}");
    let offered = completions(after, "rooms.");
    assert_eq!(offered, ["find", "save"]);
    let offered = completions(after, "screen.");
    assert!(offered.iter().any(|o| o == "draw_sprite"), "{offered:?}");
    let offered = completions(after, "Direction.");
    assert_eq!(offered, ["North", "East", "South", "West"]);
    let offered = completions(after, "Room.");
    for name in ["Move", "Attack", "NotFound"] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }
}

#[test]
fn completes_names_in_scope() {
    let offered = completions("let room = rooms.find(room_id)?", "ro");
    for name in [
        "room", "room_id", "screen", "act", "Room", "let", "match", "Some",
    ] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }
}
