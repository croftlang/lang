//! `croft run`: compiles a project for one target, builds the C with the system's
//! C compiler, and runs the program.

use std::env;
use std::path::Path;
use std::process::Command;

use crate::compile::{self, Output};

/// Optimised, strict C89 with every warning on, as the codegen tests build it.
const FLAGS: [&str; 6] = [
    "-std=c89",
    "-pedantic",
    "-Wall",
    "-Wextra",
    "-Wdeclaration-after-statement",
    "-O2",
];

/// Makes every warning an error for the C the compiler writes, standard adapters
/// included. The project's own native C gets the same warnings without it, so a
/// warning there is shown but doesn't stop the build.
const WARNINGS_AS_ERRORS: &str = "-Werror";

/// Runs the program and returns its exit code. Arguments after `--` go to the program.
pub fn run(arguments: &[String]) -> Result<i32, String> {
    let (ours, theirs) = match arguments.iter().position(|a| a == "--") {
        Some(split) => (&arguments[..split], &arguments[split + 1..]),
        None => (arguments, &[][..]),
    };

    let (path, target) = compile::parse_arguments(ours)?;
    let output = compile::write_c(&path, target)?;

    // `CC` picks the compiler, as it does for make; `cc` is the usual name for it.
    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_string());

    build_objects(&compiler, &output, &output.sources, true)?;

    if !output.native.is_empty() {
        build_objects(&compiler, &output, &output.native, false)?;
    }

    let file_name = format!("{}{}", output.name, env::consts::EXE_SUFFIX);
    let sources = output.sources.iter().chain(&output.native);
    let mut link = Command::new(&compiler);

    link.current_dir(&output.folder)
        .args(sources.map(String::as_str).map(object))
        .arg("-o")
        .arg(&file_name);

    // A standard adapter's C can need a library, as `CurlHttp` needs libcurl.
    for name in &output.sources {
        link.args(croft_std::links_for(name));
    }

    run_compiler(&compiler, &output, &mut link)?;

    let program = output.folder.join(&file_name);
    let ran = Command::new(&program)
        .args(theirs)
        .status()
        .map_err(|error| format!("cannot run `{}`: {error}", program.display()))?;

    // A program ended by a signal has no exit code; report it as a failure.
    Ok(ran.code().unwrap_or(1))
}

/// Compiles C files in the output folder to object files beside them, with
/// every warning an error when `strict`.
fn build_objects(
    compiler: &str,
    output: &Output,
    sources: &[String],
    strict: bool,
) -> Result<(), String> {
    let mut build = Command::new(compiler);

    build.current_dir(&output.folder).args(FLAGS);

    if strict {
        build.arg(WARNINGS_AS_ERRORS);
    }

    build.arg("-c").args(sources);

    run_compiler(compiler, output, &mut build)
}

/// Runs one step of the build, failing when the compiler does.
fn run_compiler(compiler: &str, output: &Output, command: &mut Command) -> Result<(), String> {
    let built = command
        .status()
        .map_err(|error| format!("cannot run the C compiler `{compiler}`: {error}"))?;

    if !built.success() {
        return Err(format!(
            "the C compiler `{compiler}` could not build the C in {}",
            output.folder.display()
        ));
    }

    Ok(())
}

/// The object file the compiler writes for a C file: `program.c` gives `program.o`.
fn object(source: &str) -> String {
    Path::new(source)
        .with_extension("o")
        .to_string_lossy()
        .into_owned()
}
