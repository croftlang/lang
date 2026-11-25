//! Generates C for small dn programs, builds it with the system C compiler
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

#[test]
fn runs_commands_events_policies_and_repositories() {
    run_program("game");
}

/// Each broken rule stops a debug build with a message naming it, and a
/// release build doesn't check.
#[test]
fn checks_rules_in_debug_builds() {
    if !has_c_compiler() {
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/rules");
    let output = generate(&source, "test", "rules");
    let debug = compile(
        &output,
        &["-DDN_DEBUG", "dn_runtime.c", "program.c", "-o", "debug"],
    );
    assert!(
        debug.status.success(),
        "{}",
        String::from_utf8_lossy(&debug.stderr)
    );
    let release = compile(&output, &["dn_runtime.c", "program.c", "-o", "release"]);
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
        run("debug", "shift"),
        (Some(1), "shift by less than 0 or more than 31".to_string())
    );
    assert_eq!(run("release", "argument"), (Some(0), String::new()));
    fs::remove_dir_all(output).ok();
}

#[test]
fn stops_with_the_error_when_the_domain_fails() {
    if !has_c_compiler() {
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/failing");
    let output = generate(&source, "test", "failing");
    let built = compile(&output, &["dn_runtime.c", "program.c", "-o", "program"]);
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
fn runs_the_dungeon_example_until_it_quits() {
    run_program_at(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon"),
        "dungeon_run",
    );
}

/// The scripted keys move the player, then defeat the nearest enemy, whose
/// `EnemyDefeated` event makes Dungeon drop treasure and Scoring add points.
#[test]
fn plays_the_dungeon_as_scripted() {
    if !has_c_compiler() {
        return;
    }
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon");
    let output = generate(&example, "test", "dungeon_outcome");
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/harness/dungeon_outcome.c");
    fs::copy(harness, output.join("outcome.c")).unwrap();
    let renamed = compile(&output, &["-c", "-Dmain=dn_program_main", "program.c"]);
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
            "dn_runtime.c",
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
        "status 0\nplayer 32,24\nenemies 1 treasure 1\npoints 100\n"
    );
    fs::remove_dir_all(output).ok();
}

#[test]
fn compiles_the_dungeon_example_for_every_target() {
    if !has_c_compiler() {
        return;
    }
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon");
    for target in ["test", "spectrum_48k"] {
        let output = generate(&example, target, "dungeon");
        if target == "spectrum_48k" {
            fs::copy(
                example.join("platform/spectrum.c"),
                output.join("spectrum.c"),
            )
            .unwrap();
        }
        let built = compile(&output, &["-c", "dn_runtime.c", "program.c"]);
        assert!(
            built.status.success(),
            "{target}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        fs::remove_dir_all(output).ok();
    }
}
