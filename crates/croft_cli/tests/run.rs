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

/// A `croft.toml` with two targets.
const TWO_TARGETS: &str = "\
[package]
name = \"greeting\"

[target.desktop]

[target.spectrum]
";

/// A port the domain calls, with no adapter.
const KEYBOARD: &str = "\
context Game {
  publish { Read }

  port Keyboard {
    fn pressed(): Int
  }

  service Read(): Int {
    calls { Keyboard }
  } = Keyboard.pressed()
}

domain Arcade {
  contexts { Game }
  use { Read } from Game
} = {
  let pressed = Read()
}
";

/// A context with nothing to run.
const LIBRARY: &str = "\
context Sums {
  fn add(a: Int, b: Int): Int = a + b
}
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
    project_with(name, source, Some(MANIFEST))
}

/// Writes a project with one source file, `app.croft`, and `manifest` as its
/// `croft.toml` when given, to a fresh folder.
fn project_with(name: &str, source: &str, manifest: Option<&str>) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("croft_cli_run_{name}_{}", std::process::id()));
    fs::remove_dir_all(&folder).ok();
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("app.croft"), source).unwrap();

    if let Some(manifest) = manifest {
        fs::write(folder.join("croft.toml"), manifest).unwrap();
    }

    folder
}

/// Runs `croft run` on a folder with the given arguments, with `CC` unset.
fn croft_run(folder: &PathBuf, arguments: &[&str]) -> Output {
    croft("run", folder, arguments)
}

/// Runs a `croft` command on a path with the given arguments, with `CC` unset.
fn croft(command: &str, path: &PathBuf, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_croft"))
        .arg(command)
        .arg(path)
        .args(arguments)
        .env_remove("CC")
        .output()
        .expect("croft runs")
}

/// What a command wrote to standard error.
fn errors(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
fn asks_for_a_target_when_there_are_several() {
    let folder = project_with("no_target", HELLO, Some(TWO_TARGETS));
    let ran = croft_run(&folder, &[]);
    assert!(!ran.status.success());

    let errors = String::from_utf8_lossy(&ran.stderr);

    assert!(
        errors.contains("choose a target with `--target`"),
        "{errors}"
    );

    fs::remove_dir_all(folder).ok();
}

#[test]
fn runs_the_only_target_without_being_told() {
    if !has_c_compiler() {
        return;
    }

    let folder = project("only_target", HELLO);
    let ran = croft_run(&folder, &[]);

    assert!(ran.status.success(), "{}", errors(&ran));
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Hello, World\n");
    fs::remove_dir_all(folder).ok();
}

#[test]
fn runs_a_project_without_a_croft_toml_on_desktop() {
    if !has_c_compiler() {
        return;
    }

    let folder = project_with("no_manifest", HELLO, None);
    let ran = croft_run(&folder, &[]);

    assert!(ran.status.success(), "{}", errors(&ran));
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Hello, World\n");

    // Named after `domain Application`.
    let program = format!("application{}", std::env::consts::EXE_SUFFIX);
    assert!(folder.join("target/croft/desktop").join(program).is_file());
    fs::remove_dir_all(folder).ok();
}

#[test]
fn runs_a_file_with_the_croft_toml_beside_it() {
    if !has_c_compiler() {
        return;
    }

    let folder = project("one_file", HELLO);
    let ran = croft_run(&folder.join("app.croft"), &[]);

    assert!(ran.status.success(), "{}", errors(&ran));

    // Named by the `croft.toml`, not the domain.
    let program = format!("greeting{}", std::env::consts::EXE_SUFFIX);
    assert!(folder.join("target/croft/desktop").join(program).is_file());
    fs::remove_dir_all(folder).ok();
}

#[test]
fn says_when_an_error_comes_from_the_default_croft_toml() {
    let folder = project_with("default_error", KEYBOARD, None);
    let checked = croft("check", &folder, &[]);

    assert!(!checked.status.success());

    let expected = "error: target `desktop` has no adapter for port `Keyboard`\n  \
                    note: there is no croft.toml, so the program runs its domain on one \
                    `desktop` target with no adapters; add a croft.toml to change that";
    assert!(errors(&checked).contains(expected), "{}", errors(&checked));
    fs::remove_dir_all(folder).ok();
}

#[test]
fn checks_contexts_with_nothing_to_run_but_does_not_run_them() {
    let folder = project_with("library", LIBRARY, None);

    let checked = croft("check", &folder, &[]);
    assert!(checked.status.success(), "{}", errors(&checked));

    let ran = croft_run(&folder, &[]);
    assert!(!ran.status.success());
    assert!(
        errors(&ran).contains("there is nothing to run: declare a `domain`"),
        "{}",
        errors(&ran)
    );

    fs::remove_dir_all(folder).ok();
}
