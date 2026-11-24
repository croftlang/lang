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

#[test]
fn runs_the_dungeon_example_until_it_quits() {
    run_program_at(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dungeon"),
        "dungeon_run",
    );
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
