//! Generates C for small Croft programs, builds it with the system C compiler
//! in strict C89 mode, and runs it. Each program's start service fails, and
//! so exits with 1, if any of its checks gives the wrong answer.

mod support;

use std::fs;
use std::path::Path;

use support::{compile, generate, has_c_compiler, run_program, run_program_at};

#[test]
fn runs_values_enums_lists_and_strings() {
    run_program("values");
}

/// A domain calling one function, printing through the built-in `Console`.
#[test]
fn prints_through_the_console() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/hello");
    let output = generate(&source, "test", "hello");
    let built = compile(&output, &["croft_runtime.c", "program.c", "-o", "program"]);

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");

    assert_eq!(String::from_utf8_lossy(&ran.stdout), "5\n");
    fs::remove_dir_all(output).ok();
}

/// Std.Files through StdioFiles: the file is written, read back and removed,
/// and reading it again stops the program with `NotFound`.
#[test]
fn reads_and_writes_files() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/files");
    let output = generate(&source, "test", "files");

    let built = compile(
        &output,
        &[
            "croft_runtime.c",
            "croft_files.c",
            "program.c",
            "-o",
            "program",
        ],
    );

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("program"))
        .current_dir(&output)
        .output()
        .expect("it runs");

    assert_eq!(ran.status.code(), Some(1));

    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "first line, then more\nStd.Files.NotFound\n"
    );

    assert!(!output.join("notes.txt").exists());
    fs::remove_dir_all(output).ok();
}

/// A shelf saved through FileStorage comes back the same, a missing one is
/// `NotFound`, and a damaged file is `StorageDamaged`.
#[test]
fn saves_and_finds_aggregates_in_files() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/storage");
    let output = generate(&source, "test", "storage");

    let built = compile(
        &output,
        &[
            "-DCROFT_DEBUG",
            "croft_runtime.c",
            "croft_files.c",
            "program.c",
            "-o",
            "program",
        ],
    );

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("program"))
        .current_dir(&output)
        .output()
        .expect("it runs");

    assert_eq!(String::from_utf8_lossy(&ran.stdout), "ok\n");
    assert!(ran.status.success());
    fs::remove_dir_all(output).ok();
}

#[test]
fn keeps_aggregates_through_adapters_of_the_programs_own() {
    run_program("repository_adapter");
}

#[test]
fn makes_an_instance_of_each_generic_for_each_type() {
    run_program("generics");
}

/// Lists get room from the `Length` on a function's result and a command's
/// parameter, not only from fields.
#[test]
fn sizes_lists_from_results_and_command_parameters() {
    run_program("sizes");
}

#[test]
fn runs_commands_events_policies_and_repositories() {
    run_program("game");
}

/// A factory holds a new board to its invariant, and its event reaches the
/// policy only when the board is saved.
#[test]
fn makes_aggregates_through_factories() {
    run_program("factories");
}

/// Each broken constraint stops a debug build with a message naming it, and a
/// release build doesn't check.
#[test]
fn checks_rules_in_debug_builds() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/constraints");
    let output = generate(&source, "test", "constraints");

    let debug = compile(
        &output,
        &[
            "-DCROFT_DEBUG",
            "croft_runtime.c",
            "program.c",
            "-o",
            "debug",
        ],
    );

    assert!(
        debug.status.success(),
        "{}",
        String::from_utf8_lossy(&debug.stderr)
    );

    let release = compile(&output, &["croft_runtime.c", "program.c", "-o", "release"]);

    assert!(
        release.status.success(),
        "{}",
        String::from_utf8_lossy(&release.stderr)
    );

    let run = |program: &str, argument: &str| {
        let ran = std::process::Command::new(output.join(program))
            .arg(argument)
            .output()
            .expect("it runs");

        (
            ran.status.code(),
            String::from_utf8_lossy(&ran.stdout).trim().to_string(),
        )
    };

    assert_eq!(run("debug", "none"), (Some(0), String::new()));

    assert_eq!(
        run("debug", "argument"),
        (
            Some(1),
            "heal: amount must satisfy Range(1, 50)".to_string()
        )
    );

    assert_eq!(
        run("debug", "result"),
        (Some(1), "half: the result must satisfy Even".to_string())
    );

    assert_eq!(
        run("debug", "field"),
        (
            Some(1),
            "Dose: amount must satisfy Range(1, 50)".to_string()
        )
    );

    assert_eq!(
        run("debug", "each"),
        (Some(1), "count: numbers must satisfy each Even".to_string())
    );

    assert_eq!(
        run("debug", "score"),
        (
            Some(1),
            "Score: value must satisfy Range(0, 999)".to_string()
        )
    );

    assert_eq!(
        run("debug", "pay"),
        (
            Some(1),
            "pay: amount must satisfy Range(0.01, 99.99)".to_string()
        )
    );

    assert_eq!(
        run("debug", "shift"),
        (Some(1), "shift by less than 0 or more than 31".to_string())
    );

    assert_eq!(
        run("debug", "name"),
        (
            Some(1),
            "Patient: name is longer than 8 characters".to_string()
        )
    );

    assert_eq!(
        run("debug", "join"),
        (
            Some(1),
            "joined text is longer than a string can hold".to_string()
        )
    );

    assert_eq!(run("release", "argument"), (Some(0), String::new()));
    assert_eq!(run("release", "name"), (Some(0), String::new()));
    fs::remove_dir_all(output).ok();
}

#[test]
fn stops_with_the_error_when_the_domain_fails() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/failing");
    let output = generate(&source, "test", "failing");
    let built = compile(&output, &["croft_runtime.c", "program.c", "-o", "program"]);

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("the program runs");

    assert_eq!(ran.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "Shop.Closed");
    fs::remove_dir_all(output).ok();
}

#[test]
fn runs_the_logistics_fixture() {
    run_program_at(&logistics(), "logistics_run");
}

fn logistics() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/logistics")
}

/// The scripted reports take the cargo from Hong Kong to Helsinki along the
/// route Routing found. Each handling event registered reaches the cargo
/// through the `UpdateDelivery` policy, until it is claimed.
#[test]
fn ships_the_cargo_as_scripted() {
    if !has_c_compiler() {
        return;
    }

    let output = generate(&logistics(), "test", "logistics_outcome");
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/harness/logistics_outcome.c");
    fs::copy(harness, output.join("outcome.c")).unwrap();

    let renamed = compile(&output, &["-c", "-Dmain=croft_program_main", "program.c"]);

    assert!(
        renamed.status.success(),
        "{}",
        String::from_utf8_lossy(&renamed.stderr)
    );

    let built = compile(
        &output,
        &[
            "-I.",
            "outcome.c",
            "program.o",
            "croft_runtime.c",
            "-o",
            "outcome",
        ],
    );

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("outcome"))
        .output()
        .expect("the harness runs");

    let printed = String::from_utf8_lossy(&ran.stdout);

    assert_eq!(
        printed,
        "Claimed at FIHEL\nRouted\nstatus 0\nlegs 2\nclaimed 1 at FIHEL\nmisdirected 0\nlast event 1 on day 10\n"
    );

    fs::remove_dir_all(output).ok();
}

#[test]
fn compiles_the_logistics_fixture_for_every_target() {
    if !has_c_compiler() {
        return;
    }

    let fixture = logistics();

    for target in ["test", "desktop"] {
        let output = generate(&fixture, target, "logistics");
        let mut files = vec!["-c", "croft_runtime.c", "program.c"];

        if target == "desktop" {
            fs::copy(fixture.join("platform/desktop.c"), output.join("desktop.c")).unwrap();
            files.extend(["croft_files.c", "desktop.c"]);
        }

        let built = compile(&output, &files);

        assert!(
            built.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&built.stderr)
        );

        fs::remove_dir_all(output).ok();
    }
}

/// Repositories that declare their own operations: finds, a save and a
/// remove that Storage provides, and a query an adapter written for the
/// repository answers.
#[test]
fn runs_operations_a_repository_declares() {
    run_program("repository_operations");
}

/// Literals and string parameters passed as borrowed views, and copied only
/// when kept.
#[test]
fn borrows_strings_passed_into_functions() {
    run_program("borrowed_strings");
}

/// A `match` of string literals writes each straight into the result, with
/// no temporary string for any arm.
#[test]
fn writes_string_literals_straight_into_results() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/borrowed_strings");
    let output = generate(&source, "test", "borrowed_strings_c");
    let c = fs::read_to_string(output.join("program.c")).expect("program.c");

    let start = c.find("void Words_name_of(").expect("name_of");
    let body = &c[start..start + c[start..].find("\n}\n").expect("its end")];

    assert!(
        body.contains("croft_string_set(\"circle\", CROFT_OUT((*out)));"),
        "{body}"
    );

    assert!(!body.contains("croft_string t"), "{body}");
    assert!(c.contains("croft_string_view_from(\"square\", 6)"));

    fs::remove_dir_all(output).ok();
}

/// Two or more Bool fields packed into bits: read, set, compared and saved.
#[test]
fn packs_bool_fields_into_bits() {
    run_program("flags");

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/flags");
    let output = generate(&source, "test", "flags_c");
    let header = fs::read_to_string(output.join("program.h")).expect("program.h");

    assert!(header.contains("croft_u8 croft_flags[2];"), "{header}");
    assert!(!header.contains("croft_bool has_key;"), "{header}");

    fs::remove_dir_all(output).ok();
}

/// `croft size`: what each kept type takes, the largest frames, what sizes
/// working strings, and whole numbers that a Range would shrink.
#[test]
fn reports_sizes_for_a_target() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/repository_operations");
    let program = support::check_program(&source);
    let report = croft_codegen::size_report(&program, "test").expect("a report");

    assert!(report.contains("Shop.Shelf  (saved:"), "{report}");
    assert!(report.contains("Largest stack frames"), "{report}");
    assert!(report.contains("  Till.cash\n"), "{report}");
    assert!(!report.contains("Shelf.stock"), "{report}");
}
