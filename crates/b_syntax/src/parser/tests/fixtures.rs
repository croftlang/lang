use std::fs;
use std::path::{Path, PathBuf};

use super::parse_ok;

fn find_b_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).expect("examples folder") {
        let path = entry.expect("folder entry").path();
        if path.is_dir() {
            find_b_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "b") {
            found.push(path);
        }
    }
}

#[test]
fn parses_every_example_file() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files = Vec::new();
    find_b_files(&examples, &mut files);
    assert!(files.len() > 5, "expected the dungeon example files");
    for path in files {
        let source = fs::read_to_string(&path).expect("example file");
        let file = std::panic::catch_unwind(|| parse_ok(&source));
        assert!(file.is_ok(), "failed to parse {}", path.display());
    }
}
