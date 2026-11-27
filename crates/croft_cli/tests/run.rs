//! `croft run` on small projects written to a temporary folder: it builds the C
//! with the system's C compiler, runs the program, and passes on its exit code.
//! Skipped where there is no C compiler.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A `croft.toml` with one target, `desktop`.
const MANIFEST: &str = "\
[package]
name = \"greeting\"

[target.desktop]
";

/// A domain that prints a greeting through `Console`.
const HELLO: &str = "\
context Greeter {
  publish { greet }

  fn greet(): String where Length(0, 12) = {
    \"Hello, World\"
  }
}

domain Application {
  contexts { Greeter }
  use { greet } from Greeter
  calls { Console }
} = {
  Console.print(greet())
}
";

/// A domain whose body fails, so the program exits with 1.
const FAILING: &str = "\
context Shop {
  publish { Open }

  error Closed

  service Open(): Result<Unit, Closed> = Err(Closed)
}

domain Market {
  contexts { Shop }
  use { Open } from Shop
} = {
  Open()?
}
";

/// Whether a C compiler named `cc` is installed.
fn has_c_compiler() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Writes a project with one source file and `croft.toml` to a fresh folder.
fn project(name: &str, source: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("croft_cli_run_{name}_{}", std::process::id()));
    fs::remove_dir_all(&folder).ok();
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("app.croft"), source).unwrap();
    fs::write(folder.join("croft.toml"), MANIFEST).unwrap();
    folder
}

/// Runs `croft run` on a folder with the given arguments, with `CC` unset.
fn croft_run(folder: &PathBuf, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_croft"))
        .arg("run")
        .arg(folder)
        .args(arguments)
        .env_remove("CC")
        .output()
        .expect("croft runs")
}

#[test]
fn builds_and_runs_the_program() {
    if !has_c_compiler() {
        return;
    }

    let folder = project("hello", HELLO);
    let ran = croft_run(&folder, &["--target", "desktop", "--", "ignored"]);

    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Hello, World\n");

    let program = format!("greeting{}", std::env::consts::EXE_SUFFIX);
    assert!(folder.join("target/croft/desktop").join(program).is_file());
    fs::remove_dir_all(folder).ok();
}

#[test]
fn exits_with_the_program_exit_code() {
    if !has_c_compiler() {
        return;
    }

    let folder = project("failing", FAILING);
    let ran = croft_run(&folder, &["--target=desktop"]);

    assert_eq!(
        ran.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    fs::remove_dir_all(folder).ok();
}

#[test]
fn reports_a_c_compiler_that_fails() {
    let folder = project("no_compiler", HELLO);

    let ran = Command::new(env!("CARGO_BIN_EXE_croft"))
        .arg("run")
        .arg(&folder)
        .args(["--target", "desktop"])
        .env("CC", "false")
        .output()
        .expect("croft runs");

    assert!(!ran.status.success());

    let errors = String::from_utf8_lossy(&ran.stderr);

    assert!(
        errors.contains("the C compiler `false` could not build"),
        "{errors}"
    );

    fs::remove_dir_all(folder).ok();
}

#[test]
fn asks_for_a_target() {
    let folder = project("no_target", HELLO);
    let ran = croft_run(&folder, &[]);
    assert!(!ran.status.success());

    let errors = String::from_utf8_lossy(&ran.stderr);

    assert!(
        errors.contains("choose a target with `--target`"),
        "{errors}"
    );

    fs::remove_dir_all(folder).ok();
}
