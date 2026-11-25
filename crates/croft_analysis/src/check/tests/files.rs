use std::fs;
use std::path::{Path, PathBuf};

use super::check_files;

fn find_source_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).expect("fixture folder") {
        let path = entry.expect("folder entry").path();
        if path.is_dir() {
            find_source_files(&path, found);
        } else if croft_syntax::is_source_file(&path) {
            found.push(path);
        }
    }
}

#[test]
fn accepts_the_logistics_fixture() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/logistics");
    let mut paths = Vec::new();
    find_source_files(&folder, &mut paths);
    let sources: Vec<String> = paths
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect();
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    let program = check_files(&sources).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert_eq!(
        program
            .contexts
            .values()
            .filter(|c| !c.is_domain && !c.is_standard)
            .count(),
        3
    );
    assert_eq!(program.domains[0].name.name, "Logistics");
}

#[test]
fn merges_an_aggregate_split_across_files() {
    let shared = "context Dungeon\nidentity RoomId: Int\n";
    let fields = "context Dungeon\naggregate Room\nidentity id: RoomId\nlives: Int\n";
    let commands = "context Dungeon\naggregate Room\ncommand Reset = {\n    set lives = 3\n}\n";
    let program = check_files(&[shared, fields, commands]).expect("no errors");
    let room = &program.contexts["Dungeon"].aggregates[0];
    assert_eq!(room.fields.len(), 2);
    assert_eq!(room.commands.len(), 1);
}

#[test]
fn reports_a_name_declared_in_two_files() {
    let first = "context Dungeon\nidentity RoomId: Int\n";
    let second = "context Dungeon {\n    identity RoomId: Int\n}";
    let errors = check_files(&[first, second]).err().unwrap();
    assert_eq!(errors, vec!["`RoomId` is declared more than once"]);
}

#[test]
fn reports_a_field_declared_in_two_parts_of_an_aggregate() {
    let first = "context Dungeon\naggregate Room\nidentity id: RoomId\nlives: Int\n";
    let second = "context Dungeon\nidentity RoomId: Int\naggregate Room\nlives: Int\n";
    let errors = check_files(&[first, second]).err().unwrap();
    assert_eq!(errors, vec!["field `lives` is declared more than once"]);
}
