mod bodies;
mod boundary;
mod declarations;
mod domain;
mod effects;
mod files;
mod functions;
mod manifest;
mod standard;

use b_syntax::{parse, SourceId};

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

/// A small valid context to add declarations to.
fn dungeon(extra: &str) -> String {
    format!(
        "context Dungeon {{\n\
         identity RoomId: Int where Range(1, 20)\n\
         value Position {{ x: Int  y: Int }}\n\
         {extra}\n}}"
    )
}
