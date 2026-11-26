//! Generating, building and running Croft programs for the tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use croft_syntax::{parse, SourceId};

/// Every warning is an error, and nothing past C89 is allowed.
pub const FLAGS: [&str; 7] = [
    "-std=c89",
    "-pedantic",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-Wdeclaration-after-statement",
    "-O2",
];

pub fn source_files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for entry in fs::read_dir(folder).expect("a program folder") {
        let path = entry.expect("a folder entry").path();

        if path.is_dir() {
            found.extend(source_files(&path));
        } else if croft_syntax::is_source_file(&path) {
            found.push(path);
        }
    }

    found.sort();

    found
}

/// Parses and checks the program in a folder, with its croft.toml.
pub fn check_program(folder: &Path) -> croft_analysis::Program {
    let files = source_files(folder)
        .iter()
        .enumerate()
        .map(|(index, path)| {
            parse(&fs::read_to_string(path).unwrap(), SourceId(index as u32)).expect("valid syntax")
        })
        .collect();

    let toml =
        fs::read_to_string(folder.join(croft_analysis::MANIFEST_FILE)).expect("a croft.toml");
    let manifest =
        croft_analysis::Manifest::parse(&toml, SourceId(999)).expect("a valid croft.toml");

    croft_analysis::check(files, Some(manifest)).unwrap_or_else(|errors| panic!("{errors:#?}"))
}

/// Writes the C for one program and target into a fresh folder.
pub fn generate(folder: &Path, target: &str, name: &str) -> PathBuf {
    let program = check_program(folder);

    let output = std::env::temp_dir().join(format!(
        "croft_codegen_{name}_{target}_{}",
        std::process::id()
    ));

    fs::create_dir_all(&output).unwrap();

    for file in croft_codegen::generate(&program, target).expect("generated C") {
        fs::write(output.join(&file.name), file.contents).unwrap();
    }

    output
}

pub fn has_c_compiler() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn compile(folder: &Path, extra: &[&str]) -> std::process::Output {
    Command::new("cc")
        .args(FLAGS)
        .args(extra)
        .current_dir(folder)
        .output()
        .expect("the C compiler runs")
}

pub fn run_program(name: &str) {
    run_program_at(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/programs")
            .join(name),
        name,
    );
}

/// Builds a program for the `test` target with every native file it names, and runs it.
pub fn run_program_at(source: &Path, name: &str) {
    if !has_c_compiler() {
        eprintln!("skipping {name}: no C compiler named `cc`");
        return;
    }

    let output = generate(source, "test", name);

    let built = compile(&output, &["croft_runtime.c", "program.c", "-o", "program"]);

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
