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
