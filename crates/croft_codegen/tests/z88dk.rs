//! The generated C built for a Z80, whose C `int` has only 16 bits, so a
//! `croft_int` is a `long`: z88dk's zsdcc compiler, from its Docker image,
//! builds each program for its test machine, and the `z88dk-ticks`
//! simulator runs it. Every answer must be the one the desktop build gives.
//!
//! Opt in by pulling the image, `docker pull z88dk/z88dk`; the tests are
//! skipped where docker or the image is missing. zsdcc is slow, so building
//! every program takes some minutes. z88dk's other compiler, sccz80, can't
//! return a struct, which the runtime does, so it can't build Croft's C.
//!
//! By hand, in a folder of C from `croft compile . --target test` with
//! `DRIVER` below saved as `driver.c`:
//!
//! ```sh
//! docker run --rm -i -v "$PWD":/src -w /src z88dk/z88dk sh -c \
//!   'zcc +test -compiler=sdcc -Dmain=croft_program_main -o program.bin \
//!    croft_runtime.c program.c driver.c && z88dk-ticks program.bin'
//! ```
//!
//! The simulator passes no arguments, so the driver reads them as one line
//! from the input, `echo "add 2147483647 1" | z88dk-ticks program.bin`, and
//! the program's `main` is renamed so the driver can call it.

// Shared with run.rs; this file uses only part of it.
#[allow(dead_code)]
mod support;

use std::fs;
use std::path::Path;
use std::process::Command;

use support::{compile, generate, has_c_compiler};

/// The image z88dk publishes, with zcc and z88dk-ticks.
const IMAGE: &str = "z88dk/z88dk";

/// A `main` for the simulator, which passes no arguments: it reads them from
/// one line of input, split at spaces, and calls the program's own `main`.
const DRIVER: &str = r#"#undef main
#include <stdio.h>
#include <string.h>

int croft_program_main(int argc, char **argv);

int main(void)
{
    static char line[128];
    static char *words[8];
    int count = 1;
    char *word;
    words[0] = "program";
    if (fgets(line, sizeof line, stdin) != NULL) {
        for (word = strtok(line, " \r\n"); word != NULL && count < 8; word = strtok(NULL, " \r\n")) {
            words[count++] = word;
        }
    }
    return croft_program_main(count, words);
}
"#;

/// Builds the C in `folder` as `program.bin` and runs it once for each line
/// of `cases.txt`, keeping what the run printed in `printed_N` and how it
/// exited in `exited_N`, for the line numbered N from 1.
const BUILD_AND_RUN: &str = "zcc +test -compiler=sdcc -Dmain=croft_program_main \
     -o program.bin croft_runtime.c program.c driver.c || exit 1
number=0
while IFS= read -r line; do
  number=$((number + 1))
  printf '%s\\n' \"$line\" | z88dk-ticks program.bin > printed_$number
  echo $? > exited_$number
done < cases.txt";

/// The programs in `tests/programs` that need no files or network, and
/// that zsdcc builds in a minute or two. `values`, `more_lists`, `more_text`,
/// `bounded_lists` and `bounded_bytes` run too, but take zsdcc from several
/// minutes to a quarter of an hour each.
const PROGRAMS: [&str; 15] = [
    "borrowed_strings",
    "bounded_strings",
    "early_returns",
    "factories",
    "failing_constraints",
    "flags",
    "game",
    "generics",
    "narrow_numbers",
    "repository_adapter",
    "repository_operations",
    "results",
    "sizes",
    "text_codes",
    "text_numbers",
];

/// The arguments for each run of the `arithmetic` program, the same as its
/// desktop test in run.rs gives.
const ARITHMETIC: [&str; 51] = [
    "add 2147483646 1",
    "add 2147483647 1",
    "subtract -2147483648 1",
    "multiply -65536 32768",
    "multiply 46341 46341",
    "multiply 65536 65536",
    "divide -7 2",
    "divide 10 0",
    "divide -2147483648 -1",
    "remainder -7 2",
    "remainder 5 0",
    "remainder -2147483648 -1",
    "negate -2147483648",
    "abs -2147483648",
    "shift 1 31",
    "sum 2147483647 1",
    "small_divide -7 2",
    "small_divide 7 2",
    "small_divide_negative -7 -2",
    "small_divide_negative 7 -2",
    "small_remainder -7 -2",
    "small_remainder 7 -2",
    "decimal_add 21474836 1",
    "decimal_add 30000000 0",
    "decimal_subtract -21474836 1",
    "decimal_negate 5 2",
    "decimal_times 1000 -3",
    "decimal_times 21474836 2",
    "decimal_multiply 50000 50000",
    "decimal_multiply 40000 500",
    "decimal_multiply -3 7",
    "decimal_divide 1 3",
    "decimal_divide 2 0",
    "add -1 -2147483647",
    "clamp 50 0 10",
    "clamp -5 0 10",
    "clamp 5 10 0",
    "pow 3 4",
    "pow 7 0",
    "pow -2 31",
    "pow 2 31",
    "pow 65536 2",
    "pow 2 -1",
    "sign -9",
    "sign 0",
    "checked_add 2 3",
    "checked_add 2147483647 1",
    "checked_subtract -2147483648 1",
    "checked_multiply 65536 32768",
    "checked_multiply -65536 32768",
    "decimal_clamp 0 3",
];

/// Whether docker runs and already has the z88dk image.
fn has_z88dk() -> bool {
    Command::new("docker")
        .args(["image", "inspect", IMAGE])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Builds the C in `folder` with zsdcc and runs it once for each of `cases`,
/// giving how each run exited and what it printed.
fn run_on_z88dk(folder: &Path, cases: &[&str]) -> Vec<(Option<i32>, String)> {
    fs::write(folder.join("driver.c"), DRIVER).expect("the driver");
    fs::write(folder.join("cases.txt"), cases.join("\n") + "\n").expect("the cases");

    let mount = format!("{}:/src", folder.display());

    let built = Command::new("docker")
        .args(["run", "--rm", "-v", &mount, "-w", "/src", IMAGE])
        .args(["sh", "-c", BUILD_AND_RUN])
        .output()
        .expect("docker runs");

    assert!(
        built.status.success(),
        "{}{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    (1..=cases.len())
        .map(|number| {
            let exited = fs::read_to_string(folder.join(format!("exited_{number}")))
                .expect("how the run exited");
            let printed = fs::read_to_string(folder.join(format!("printed_{number}")))
                .expect("what the run printed");

            (
                exited.trim().parse().ok(),
                what_the_program_printed(&printed),
            )
        })
        .collect()
}

/// What the program printed in what the simulator prints: after the line
/// it echoes as it reads the input, and before the count of ticks it took.
fn what_the_program_printed(printed: &str) -> String {
    let (_, after_echo) = printed.split_once('\n').unwrap_or(("", printed));

    let before_ticks = match after_echo.rfind("Ticks:") {
        Some(at) => &after_echo[..at],
        None => after_echo,
    };

    before_ticks.trim().to_string()
}

/// Builds the C in `folder` with the system C compiler and runs it once for
/// each of `cases`, giving how each run exited and what it printed.
fn run_on_desktop(folder: &Path, cases: &[&str]) -> Vec<(Option<i32>, String)> {
    let built = compile(folder, &["croft_runtime.c", "program.c", "-o", "program"]);

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    cases
        .iter()
        .map(|case| {
            let ran = Command::new(folder.join("program"))
                .args(case.split(' '))
                .output()
                .expect("it runs");

            // z88dk-ticks prints the error output with the rest, so the
            // desktop's two are read together: failures come last.
            let printed = format!(
                "{}{}",
                String::from_utf8_lossy(&ran.stdout),
                String::from_utf8_lossy(&ran.stderr)
            );

            (ran.status.code(), printed.trim().to_string())
        })
        .collect()
}

/// Whole-number and decimal arithmetic, with checks on and off, gives the
/// same answers and stops with the same messages where `int` has 16 bits as
/// on the desktop.
#[test]
fn works_out_whole_numbers_in_32_bits_where_int_has_16_bits() {
    if !has_c_compiler() || !has_z88dk() {
        eprintln!("skipping: no C compiler, or no docker with the {IMAGE} image");
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/arithmetic");

    let builds = [
        ("test", "arithmetic_z88dk_desktop", "arithmetic_z88dk"),
        (
            "unchecked",
            "arithmetic_unchecked_z88dk_desktop",
            "arithmetic_unchecked_z88dk",
        ),
    ];

    for (target, desktop_name, z88dk_name) in builds {
        let desktop = generate(&source, target, desktop_name);
        let z80 = generate(&source, target, z88dk_name);

        let expected = run_on_desktop(&desktop, &ARITHMETIC);
        let found = run_on_z88dk(&z80, &ARITHMETIC);

        for ((case, expected), found) in ARITHMETIC.iter().zip(expected).zip(found) {
            assert_eq!(found, expected, "{case} for the {target} target");
        }

        fs::remove_dir_all(desktop).ok();
        fs::remove_dir_all(z80).ok();
    }
}

/// Each program's checks pass where `int` has 16 bits, as they do on the
/// desktop.
#[test]
fn runs_programs_where_int_has_16_bits() {
    if !has_z88dk() {
        eprintln!("skipping: no docker with the {IMAGE} image");
        return;
    }

    let programs = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");

    for name in PROGRAMS {
        let output = generate(&programs.join(name), "test", &format!("{name}_z88dk"));
        let ran = run_on_z88dk(&output, &[""]);

        assert_eq!(ran[0].0, Some(0), "{name}: a check failed: {}", ran[0].1);
        fs::remove_dir_all(output).ok();
    }
}
