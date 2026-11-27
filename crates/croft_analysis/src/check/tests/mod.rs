//! Tests for the checker, and helpers that check sources and give back their errors.

mod arithmetic;
mod bodies;
mod boundary;
mod calls;
mod declarations;
mod domain;
mod effects;
mod factories;
mod failing_constraints;
mod files;
mod functions;
mod known_constraints;
mod manifest;
mod numbers;
mod references;
mod standard;

use croft_syntax::{parse, SourceId};

use crate::program::Program;

/// Checks several files as one program.
fn check_files(sources: &[&str]) -> Result<Program, Vec<String>> {
    let files = sources
        .iter()
        .enumerate()
        .map(|(index, source)| parse(source, SourceId(index as u32)).expect("valid syntax"))
        .collect();

    super::check(files, None)
        .map_err(|errors| errors.into_iter().map(|error| error.message).collect())
}

/// The errors checking `source` reports, or none.
fn errors(source: &str) -> Vec<String> {
    match check_files(&[source]) {
        Ok(_) => Vec::new(),
        Err(errors) => errors,
    }
}

/// Asserts that checking `source` reports exactly `expected`.
fn assert_error(source: &str, expected: &str) {
    assert_eq!(errors(source), vec![expected.to_string()]);
}

/// A small valid context to add declarations to. It has a bare `Room` for
/// `RoomId` to identify, unless `extra` declares its own.
fn dungeon(extra: &str) -> String {
    let room = if extra.contains("aggregate Room") {
        ""
    } else {
        "aggregate Room { id: RoomId }"
    };

    format!(
        "context Dungeon {{\n\
         identity RoomId for Room: Int where Range(1, 20)\n\
         value Position {{ x: Int  y: Int }}\n\
         {room}\n\
         {extra}\n}}"
    )
}
