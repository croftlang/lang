//! Generates C for small dn programs, builds it with the system C compiler
//! in strict C89 mode, and runs it. Each program's start service fails, and
//! so exits with 1, if any of its checks gives the wrong answer.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dn_syntax::{parse, SourceId};

/// Every warning is an error, and nothing past C89 is allowed.
const FLAGS: [&str; 7] = [
    "-std=c89",
    "-pedantic",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-Wdeclaration-after-statement",
    "-O2",
];

fn dn_files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).expect("a program folder") {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            found.extend(dn_files(&path));
        } else if path.extension().is_some_and(|e| e == "dn") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Writes the C for one program and target into a fresh folder.
fn generate(folder: &Path, target: &str, name: &str) -> PathBuf {
    let files = dn_files(folder)
        .iter()
        .enumerate()
        .map(|(index, path)| {
            parse(&fs::read_to_string(path).unwrap(), SourceId(index as u32)).expect("valid syntax")
        })
        .collect();
    let program = dn_analysis::check(files).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let output =
        std::env::temp_dir().join(format!("dn_codegen_{name}_{target}_{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    for file in dn_codegen::generate(&program, target).expect("generated C") {
        fs::write(output.join(&file.name), file.contents).unwrap();
    }
    output
}

fn has_c_compiler() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn compile(folder: &Path, extra: &[&str]) -> std::process::Output {
    Command::new("cc")
        .args(FLAGS)
        .args(extra)
        .current_dir(folder)
        .output()
        .expect("the C compiler runs")
}

fn run_program(name: &str) {
    if !has_c_compiler() {
        eprintln!("skipping {name}: no C compiler named `cc`");
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/programs")
        .join(name);
    let output = generate(&source, "test", name);
    let built = compile(
        &output,
        &["-DDN_DEBUG", "dn_runtime.c", "program.c", "-o", "program"],
    );
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(output.join("program"))
        .output()
        .expect("the program runs");
    assert!(
        ran.status.success(),
        "a check failed: {}",
        String::from_utf8_lossy(&ran.stdout)
    );
    fs::remove_dir_all(output).ok();
}

#[test]
fn runs_values_enums_lists_and_strings() {
    run_program("values");
}

#[test]
fn runs_commands_events_policies_and_repositories() {
    run_program("game");
}

#[test]
fn compiles_the_dungeon_example_for_every_target() {
    if !has_c_compiler() {
        return;
    }
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon");
    for target in ["test", "spectrum_48k"] {
        let output = generate(&example, target, "dungeon");
        let built = compile(&output, &["-c", "dn_runtime.c", "program.c"]);
        assert!(
            built.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        fs::remove_dir_all(output).ok();
    }
}
