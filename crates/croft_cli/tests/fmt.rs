//! `croft fmt` on files written to a temporary folder.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A source file that `croft fmt` would change.
const MESSY: &str = "context Greeter {\n  fn greet( ):String={\n\"Hello\"\n  }\n}\n";

/// `MESSY` as `croft fmt` writes it.
const TIDY: &str = "context Greeter {\n    fn greet(): String = {\n        \"Hello\"\n    }\n}\n";

/// A fresh folder holding one source file with the given text.
fn folder_with(name: &str, source: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("croft_cli_fmt_{name}_{}", std::process::id()));
    fs::remove_dir_all(&folder).ok();
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("app.croft"), source).unwrap();
    folder
}

/// Runs `croft fmt` on a folder with the given arguments.
fn croft_fmt(folder: &PathBuf, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_croft"))
        .arg("fmt")
        .arg(folder)
        .args(arguments)
        .output()
        .expect("croft runs")
}

#[test]
fn check_lists_unformatted_files_and_leaves_them_alone() {
    let folder = folder_with("check", MESSY);
    let checked = croft_fmt(&folder, &["--check"]);

    assert!(!checked.status.success());
    assert!(String::from_utf8_lossy(&checked.stdout).contains("app.croft"));
    assert_eq!(fs::read_to_string(folder.join("app.croft")).unwrap(), MESSY);

    fs::remove_dir_all(folder).ok();
}

#[test]
fn formats_files_in_place() {
    let folder = folder_with("write", MESSY);
    let formatted = croft_fmt(&folder, &[]);

    assert!(
        formatted.status.success(),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );

    assert_eq!(fs::read_to_string(folder.join("app.croft")).unwrap(), TIDY);
    assert!(croft_fmt(&folder, &["--check"]).status.success());

    fs::remove_dir_all(folder).ok();
}

#[test]
fn reports_a_file_that_does_not_parse() {
    let folder = folder_with("broken", "context Greeter {\n");
    let formatted = croft_fmt(&folder, &[]);

    assert!(!formatted.status.success());
    assert!(String::from_utf8_lossy(&formatted.stderr).contains("app.croft:2:1: error:"));

    fs::remove_dir_all(folder).ok();
}
