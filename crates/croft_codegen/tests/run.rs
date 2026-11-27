//! Generates C for small Croft programs, builds it with the system C compiler
//! in strict C89 mode, and runs it. Each program's main service fails, and
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

/// Std.Random through SeededRandom: thousands of rolls each land in their
/// range, whole range included, and every side of a die comes up. With
/// `low` above `high`, known only while running, the program stops.
#[test]
fn rolls_within_range() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/random");
    let output = generate(&source, "test", "random");

    let built = compile(
        &output,
        &[
            "croft_runtime.c",
            "croft_random.c",
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

    let run = |arguments: &[&str]| {
        std::process::Command::new(output.join("program"))
            .args(arguments)
            .output()
            .expect("it runs")
    };

    let rolled = run(&[]);
    assert_eq!(String::from_utf8_lossy(&rolled.stdout), "ok\n");
    assert!(rolled.status.success());

    let once = run(&["4", "4"]);
    assert_eq!(String::from_utf8_lossy(&once.stdout), "4\n");

    let reversed = run(&["6", "1"]);
    assert_eq!(reversed.status.code(), Some(1));

    assert_eq!(
        String::from_utf8_lossy(&reversed.stdout),
        "SeededRandom.between: low <= high must hold\n"
    );

    fs::remove_dir_all(output).ok();
}

/// `Files.list` through StdioFiles: the names in a folder in order, each
/// folder's ending in `/`; `NoRoom` for more names than a list holds, and
/// `NotFound` for a folder that isn't there.
#[test]
fn lists_the_names_in_a_folder() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/folders");
    let output = generate(&source, "test", "folders");

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

    let pages = output.join("pages");
    fs::create_dir_all(pages.join("drafts")).expect("a folder");

    for name in ["b.md", "a.md", "Z.md"] {
        fs::write(pages.join(name), "").expect("a file");
    }

    // One more than a `List<String>` holds.
    let many = output.join("many");
    fs::create_dir_all(&many).expect("a folder");

    for index in 0..17 {
        fs::write(many.join(format!("{index}.md")), "").expect("a file");
    }

    let run = |folder: &str| {
        std::process::Command::new(output.join("program"))
            .arg(folder)
            .current_dir(&output)
            .output()
            .expect("it runs")
    };

    let listed = run("pages");
    assert_eq!(listed.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&listed.stdout),
        "Z.md\na.md\nb.md\ndrafts/\n"
    );

    let full = run("many");
    assert_eq!(full.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&full.stdout), "Std.Files.NoRoom\n");

    let missing = run("missing");
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&missing.stdout),
        "Std.Files.NotFound\n"
    );

    fs::remove_dir_all(output).ok();
}

/// `Files.make_folder` makes each folder on the way and leaves one that is
/// already there, and `Files.copy` copies a file longer than a `Bytes`
/// holds; copying one that isn't there is `NotFound`.
#[test]
fn makes_folders_and_copies_files() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/copies");
    let output = generate(&source, "test", "copies");

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

    let big: Vec<u8> = (0..1000).map(|index| (index % 251) as u8).collect();
    fs::write(output.join("big.bin"), &big).expect("a file");

    let run = |file: &str| {
        std::process::Command::new(output.join("program"))
            .arg(file)
            .current_dir(&output)
            .output()
            .expect("it runs")
    };

    let built_site = run("big.bin");
    assert_eq!(built_site.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&built_site.stdout),
        "big.bin,posts/\n"
    );
    assert_eq!(
        fs::read(output.join("site/blog/big.bin")).expect("the copy"),
        big
    );

    let missing = run("missing.bin");
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&missing.stdout),
        "Std.Files.NotFound\n"
    );

    fs::remove_dir_all(output).ok();
}

/// StdioFiles never cuts a path short: one longer than it can pass on, or
/// with a NUL byte in it, is not there and can't be written, even when the
/// file it would be cut to is. An argument longer than a string, or more
/// arguments than a list holds, stop the program.
#[test]
fn fails_instead_of_cutting_paths_and_arguments_short() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/paths");
    let output = generate(&source, "test", "paths");

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

    let cut = "a".repeat(255);
    fs::write(output.join("a"), "kept").expect("a file");
    fs::write(output.join(&cut), "kept").expect("a file");

    let run = |arguments: &[String]| {
        std::process::Command::new(output.join("program"))
            .args(arguments)
            .current_dir(&output)
            .output()
            .expect("it runs")
    };

    let fits = run(&["b".repeat(20)]);
    assert_eq!(String::from_utf8_lossy(&fits.stdout), "written\n");
    assert_eq!(fits.status.code(), Some(0));

    let with_nul = run(&[]);
    assert_eq!(
        String::from_utf8_lossy(&with_nul.stdout),
        "Std.Files.Unreadable\n"
    );
    assert_eq!(with_nul.status.code(), Some(1));

    let too_long = run(&["a".repeat(300)]);
    assert_eq!(
        String::from_utf8_lossy(&too_long.stdout),
        "Std.Files.Unreadable\n"
    );
    assert_eq!(too_long.status.code(), Some(1));

    assert_eq!(fs::read_to_string(output.join("a")).unwrap(), "kept");
    assert_eq!(fs::read_to_string(output.join(&cut)).unwrap(), "kept");

    let long_argument = run(&["c".repeat(301)]);
    assert_eq!(
        String::from_utf8_lossy(&long_argument.stdout),
        "an argument is longer than a string can hold\n"
    );
    assert_eq!(long_argument.status.code(), Some(1));

    let many: Vec<String> = (0..17).map(|index| format!("{index}.txt")).collect();
    let too_many = run(&many);
    assert_eq!(
        String::from_utf8_lossy(&too_many.stdout),
        "there are more arguments than a list can hold\n"
    );
    assert_eq!(too_many.status.code(), Some(1));

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

/// A saved file changed so that a number no longer fits its field, a tag
/// names no variant, a flag is neither true nor false, or a constraint or
/// invariant is broken loads as `StorageDamaged`, even with checks off.
#[test]
fn finds_damaged_aggregates_in_files() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/damaged_storage");

    for target in ["test", "unchecked"] {
        let output = generate(&source, target, "damaged_storage");

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

        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            "ok\n",
            "{target}: {}",
            String::from_utf8_lossy(&ran.stderr)
        );
        assert!(ran.status.success());
        fs::remove_dir_all(output).ok();
    }
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

/// Each broken constraint stops the program with a message naming it, and a
/// target that sets `checks = false` doesn't check.
#[test]
fn checks_rules_in_every_build_by_default() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/constraints");
    let output = generate(&source, "test", "constraints");

    // No flags: a build checks its constraints unless its target turns them off.
    let checked = compile(&output, &["croft_runtime.c", "program.c", "-o", "checked"]);

    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let unchecked_output = generate(&source, "unchecked", "constraints");
    let unchecked = compile(
        &unchecked_output,
        &["croft_runtime.c", "program.c", "-o", "unchecked"],
    );

    assert!(
        unchecked.status.success(),
        "{}",
        String::from_utf8_lossy(&unchecked.stderr)
    );

    let run = |program: &str, argument: &str| {
        let folder = if program == "unchecked" {
            &unchecked_output
        } else {
            &output
        };
        let ran = std::process::Command::new(folder.join(program))
            .arg(argument)
            .output()
            .expect("it runs");

        (
            ran.status.code(),
            String::from_utf8_lossy(&ran.stdout).trim().to_string(),
        )
    };

    assert_eq!(run("checked", "none"), (Some(0), String::new()));

    assert_eq!(
        run("checked", "argument"),
        (
            Some(1),
            "heal: amount must satisfy Range(1, 50)".to_string()
        )
    );

    assert_eq!(
        run("checked", "result"),
        (Some(1), "half: the result must satisfy Even".to_string())
    );

    assert_eq!(
        run("checked", "field"),
        (
            Some(1),
            "Dose: amount must satisfy Range(1, 50)".to_string()
        )
    );

    assert_eq!(
        run("checked", "each"),
        (Some(1), "count: numbers must satisfy each Even".to_string())
    );

    assert_eq!(
        run("checked", "contract"),
        (Some(1), "move_bed: from != to must hold".to_string())
    );

    assert_eq!(
        run("checked", "score"),
        (
            Some(1),
            "Score: value must satisfy Range(0, 999)".to_string()
        )
    );

    assert_eq!(
        run("checked", "pay"),
        (
            Some(1),
            "pay: amount must satisfy Range(0.01, 99.99)".to_string()
        )
    );

    assert_eq!(
        run("checked", "shift"),
        (Some(1), "shift by less than 0 or more than 31".to_string())
    );

    assert_eq!(
        run("checked", "name"),
        (
            Some(1),
            "Patient: name is longer than 8 characters".to_string()
        )
    );

    assert_eq!(
        run("checked", "join"),
        (
            Some(1),
            "joined text is longer than a string can hold".to_string()
        )
    );

    assert_eq!(run("unchecked", "argument"), (Some(0), String::new()));
    assert_eq!(run("unchecked", "name"), (Some(0), String::new()));
    fs::remove_dir_all(output).ok();
    fs::remove_dir_all(unchecked_output).ok();
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

/// The folder of the Logistics fixture.
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

/// Strings get the room they can need, and keep their values through
/// lists, options, matches, variants and `set`.
#[test]
fn keeps_strings_in_the_size_they_can_need() {
    run_program("bounded_strings");
}

/// `split` borrows each part from the string it splits, a function gives
/// back a string as long as it can need, and a parameter's `Length` sizes
/// what is made from it.
#[test]
fn sizes_strings_from_where_they_come_from() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/bounded_strings");
    let output = generate(&source, "test", "bounded_strings_c");
    let c = fs::read_to_string(output.join("program.c")).expect("program.c");

    let start = c.find("void Words_words(").expect("words");
    let body = &c[start..start + c[start..].find("\n}\n").expect("its end")];

    // Each part is borrowed from the lowered line, and copied only to be
    // given back. A line of 12 characters splits into at most 13 parts.
    assert!(body.contains("croft_list13_string_view t"), "{body}");
    assert!(body.contains("croft_list13_text_12 *out"), "{body}");
    assert!(!body.contains("croft_string t"), "{body}");

    // Results are as long as what the body gives back, or their own `Length`.
    assert!(c.contains("void Words_noun_of(const Words_Command *p_given, croft_text_7 *out)"));
    assert!(c.contains("void Words_second(croft_string_view p_line, croft_text_23 *out)"));
    assert!(c.contains("void Words_code(croft_text_10 *out)"));
    assert!(c.contains("void Words_dots(croft_int p_count, croft_string *out)"));
    fs::remove_dir_all(output).ok();
}

/// `Bytes` kept in the size they can need, through literals, `add`, `slice`,
/// fields, variants, options, `with`, `set`, and an aggregate saved and found.
#[test]
fn keeps_bytes_in_the_size_they_can_need() {
    run_program("bounded_bytes");
}

/// A `Bytes` field, parameter or result with a `Length` below the longest is a
/// `croft_bytes_N`, and one with nothing to bound it a full `croft_bytes`.
#[test]
fn sizes_bytes_from_where_they_come_from() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/bounded_bytes");
    let output = generate(&source, "test", "bounded_bytes_c");
    let header = fs::read_to_string(output.join("program.h")).expect("program.h");

    assert!(header.contains("    croft_bytes_4 magic;"), "{header}");
    assert!(
        header.contains("            croft_bytes_2 token;"),
        "{header}"
    );
    assert!(header.contains("    croft_bytes_3 tag;"), "{header}");

    assert!(header.contains("void Packets_magic(croft_bytes_2 *out);"));
    assert!(
        header.contains("void Packets_framed(const croft_bytes_6 *p_payload, croft_bytes_7 *out);")
    );
    assert!(header
        .contains("void Packets_token_of(const Packets_Message *p_given, croft_bytes_2 *out);"));
    assert!(header.contains("void Packets_body_length(const croft_bytes *p_body, croft_int *out);"));
    assert!(header
        .contains("void Packets_wrapped(const croft_bytes_4 *p_payload, croft_bytes_6 *out);"));
    fs::remove_dir_all(output).ok();
}

/// Lists kept in the size they can need, through literals, `add`, `+`,
/// `filter`, `map`, `split`, fields, variants, `with`, `set`, and an
/// aggregate saved and found.
#[test]
fn keeps_lists_in_the_size_they_can_need() {
    run_program("bounded_lists");
}

/// A list field, parameter or result with a `Length` below the longest is a
/// `croft_listN_...`, one with nothing to bound it a full `croft_list_...`,
/// and lists of any size share one equality function for their items.
#[test]
fn sizes_lists_from_where_they_come_from() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/bounded_lists");
    let output = generate(&source, "test", "bounded_lists_c");
    let header = fs::read_to_string(output.join("program.h")).expect("program.h");

    assert!(
        header.contains("    croft_list3_Packing_Item items;"),
        "{header}"
    );
    assert!(header.contains("    croft_list4_int weights;"), "{header}");
    assert!(
        header.contains("            croft_list2_int coins;"),
        "{header}"
    );
    assert!(
        header.contains("    croft_list3_Packing_Item carrying;"),
        "{header}"
    );

    assert!(header.contains("void Packing_starter(croft_list2_Packing_Item *out);"));
    assert!(header.contains(
        "void Packing_with_flask(const croft_list3_Packing_Item *p_items, croft_list4_Packing_Item *out);"
    ));
    assert!(
        header.contains("void Packing_coins_of(const Packing_Bag *p_bag, croft_list2_int *out);")
    );
    assert!(header
        .contains("void Packing_heavy(const croft_list4_int *p_weights, croft_list4_int *out);"));
    assert!(header
        .contains("void Packing_joined(const croft_list2_int *p_first, croft_list5_int *out);"));
    assert!(
        header.contains("void Packing_words(croft_string_view p_line, croft_list10_text_9 *out);")
    );
    assert!(header.contains("void Packing_total(const croft_list_int *p_weights, croft_int *out);"));

    assert!(
        header.contains("croft_bool croft_equal_list_int(const croft_int *a, croft_u16 a_length,")
    );
    assert!(!header.contains("croft_equal_list4_int"), "{header}");
    fs::remove_dir_all(output).ok();
}

#[test]
fn writes_early_returns_straight_into_the_result() {
    run_program("early_returns");
}

/// Each early return builds its variant straight in the result, and the
/// returns share the temporaries they need.
#[test]
fn shares_temporaries_between_early_returns() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/early_returns");
    let output = generate(&source, "test", "early_returns_c");
    let c = fs::read_to_string(output.join("program.c")).expect("program.c");

    let start = c.find("void Orders_read(").expect("read");
    let body = &c[start..start + c[start..].find("\n}\n").expect("its end")];

    assert!(
        body.contains("(*out).value.tag = Orders_Order_Wait;"),
        "{body}"
    );
    assert!(!body.contains("Orders_Order t"), "{body}");
    fs::remove_dir_all(output).ok();
}

#[test]
fn works_out_narrow_numbers_as_if_they_had_32_bits() {
    run_program("narrow_numbers");
}

/// Whole numbers have exactly 32 bits. With checks on, arithmetic past them
/// and dividing by zero stop the program; with them off the answer wraps
/// around in 32 bits and dividing by zero gives 0.
#[test]
fn works_out_whole_numbers_in_32_bits() {
    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/arithmetic");
    let checked = generate(&source, "test", "arithmetic");
    let unchecked = generate(&source, "unchecked", "arithmetic_unchecked");

    for folder in [&checked, &unchecked] {
        let built = compile(folder, &["croft_runtime.c", "program.c", "-o", "program"]);

        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let run = |folder: &Path, arguments: &str| {
        let ran = std::process::Command::new(folder.join("program"))
            .args(arguments.split(' '))
            .output()
            .expect("it runs");

        (
            ran.status.code(),
            String::from_utf8_lossy(&ran.stdout).trim().to_string(),
        )
    };

    let too_big = (Some(1), "a whole number doesn't fit in 32 bits".to_string());
    let by_zero = (Some(1), "division by zero".to_string());

    // What each gives with checks on, and with them off.
    let cases = [
        (
            "add 2147483646 1",
            (Some(0), "2147483647".to_string()),
            "2147483647",
        ),
        ("add 2147483647 1", too_big.clone(), "-2147483648"),
        ("subtract -2147483648 1", too_big.clone(), "2147483647"),
        (
            "multiply -65536 32768",
            (Some(0), "-2147483648".to_string()),
            "-2147483648",
        ),
        ("multiply 46341 46341", too_big.clone(), "-2147479015"),
        ("multiply 65536 65536", too_big.clone(), "0"),
        ("divide -7 2", (Some(0), "-3".to_string()), "-3"),
        ("divide 10 0", by_zero.clone(), "0"),
        ("divide -2147483648 -1", too_big.clone(), "-2147483648"),
        ("remainder -7 2", (Some(0), "-1".to_string()), "-1"),
        ("remainder 5 0", by_zero.clone(), "0"),
        ("remainder -2147483648 -1", (Some(0), "0".to_string()), "0"),
        ("negate -2147483648", too_big.clone(), "-2147483648"),
        ("abs -2147483648", too_big.clone(), "-2147483648"),
        (
            "shift 1 31",
            (Some(0), "-2147483648".to_string()),
            "-2147483648",
        ),
        ("sum 2147483647 1", too_big.clone(), "-2147483648"),
        ("decimal_add 21474836 1", too_big.clone(), "-21474835.96"),
        ("decimal_add 30000000 0", too_big.clone(), "-12949672.96"),
        (
            "decimal_multiply 50000 50000",
            too_big.clone(),
            "8918968.32",
        ),
        (
            "decimal_multiply 40000 500",
            (Some(0), "20000000.00".to_string()),
            "20000000.00",
        ),
        (
            "decimal_multiply -3 7",
            (Some(0), "-21.00".to_string()),
            "-21.00",
        ),
        ("decimal_divide 1 3", (Some(0), "0.33".to_string()), "0.33"),
        ("decimal_divide 2 0", by_zero.clone(), "0.00"),
    ];

    for (arguments, with_checks, without) in cases {
        assert_eq!(run(&checked, arguments), with_checks, "{arguments}");

        assert_eq!(
            run(&unchecked, arguments),
            (Some(0), without.to_string()),
            "{arguments} without checks"
        );
    }

    fs::remove_dir_all(checked).ok();
    fs::remove_dir_all(unchecked).ok();
}

/// Std.Text's replace, index_of, lines, repeat, padding and join.
#[test]
fn runs_more_text_methods() {
    run_program("more_text");
}

/// to_int on text at the edges of Int's range, and past them.
#[test]
fn reads_numbers_only_as_far_as_int_reaches() {
    run_program("text_numbers");
}

/// sort, sort_by, reverse, index_of and `+` on lists.
#[test]
fn runs_more_list_methods() {
    run_program("more_lists");
}

/// is_ok, is_err, map, map_err and unwrap_or on results.
#[test]
fn runs_result_methods() {
    run_program("results");
}

/// A constraint with `or`, on a parameter or in the contract, fails the call
/// with its error, with checks on and with the target's checks turned off.
#[test]
fn fails_with_a_constraints_error() {
    run_program("failing_constraints");

    if !has_c_compiler() {
        return;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/failing_constraints");
    let output = generate(&source, "unchecked", "failing_constraints_unchecked");
    let built = compile(&output, &["croft_runtime.c", "program.c", "-o", "program"]);

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");

    assert!(
        ran.status.success(),
        "a check failed: {}",
        String::from_utf8_lossy(&ran.stdout)
    );
    fs::remove_dir_all(output).ok();
}
