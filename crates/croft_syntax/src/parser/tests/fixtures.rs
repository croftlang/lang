use std::fs;
use std::path::{Path, PathBuf};

use super::parse_ok;

fn find_source_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).expect("a folder of Croft files") {
        let path = entry.expect("folder entry").path();

        if path.is_dir() {
            find_source_files(&path, found);
        } else if crate::is_source_file(&path) {
            found.push(path);
        }
    }
}

#[test]
fn parses_every_fixture_file() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut files = Vec::new();
    find_source_files(&fixtures, &mut files);
    assert!(files.len() > 5, "expected the fixture files");

    for path in files {
        let source = fs::read_to_string(&path).expect("example file");
        let file = std::panic::catch_unwind(|| parse_ok(&source));
        assert!(file.is_ok(), "failed to parse {}", path.display());
    }
}
