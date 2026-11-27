//! The `croft` command-line tool.

mod compile;
mod diagnostics;
mod format;
mod lsp;
mod project;
mod run;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::diagnostics::print_diagnostics;
use crate::project::Project;

/// The help text, listing every command.
const USAGE: &str = "\
croft - a domain language that compiles to portable C

Usage:
    croft check [path]      Check the source files in a folder (default: .)
    croft compile [path] [--target <name>]
                        Check, then write C for one target to target/croft/<name>
    croft run [path] [--target <name>] [-- arguments]
                        Compile, build the C with $CC (default: cc), and run it
    croft size [path] [--target <name>]
                        Show how much memory types and functions take on a target
    croft fmt [path] [--check]
                        Lay out the source files in the Croft style, or with
                        --check, list the ones that are not
    croft lsp               Run the language server, for editors
    croft help              Show this message
    croft version           Show the version

--target can be left out when croft.toml has one target. Without a
croft.toml, the program runs its domain on the `desktop` target.
";

/// Runs the command named by the first argument.
fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let Some(command) = arguments.first() else {
        print!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let rest = &arguments[1..];

    let result = match command.as_str() {
        "check" => parse_path(rest).and_then(|(path, flags)| {
            reject_flags(&flags)?;
            check(&path)
        }),
        "compile" => compile::compile(rest),
        "size" => compile::size(rest),
        "fmt" => format::format(rest),
        "run" => match run::run(rest) {
            Ok(0) => Ok(()),
            Ok(code) => return ExitCode::from(u8::try_from(code).unwrap_or(1)),
            Err(message) => Err(message),
        },
        "lsp" => lsp::run(),
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("croft {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) if message.is_empty() => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Splits arguments into an optional path and `--flags`.
fn parse_path(arguments: &[String]) -> Result<(PathBuf, Vec<String>), String> {
    let mut path = None;
    let mut flags = Vec::new();

    for argument in arguments {
        if argument.starts_with("--") {
            flags.push(argument.clone());
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            return Err(format!("unexpected argument `{argument}`"));
        }
    }

    Ok((path.unwrap_or_else(|| PathBuf::from(".")), flags))
}

/// Fails on the first flag, for a command that takes none.
fn reject_flags(flags: &[String]) -> Result<(), String> {
    match flags.first() {
        Some(flag) => Err(format!("unknown option `{flag}`")),
        None => Ok(()),
    }
}

/// Loads and checks the project at `path`. Prints any diagnostics and
/// fails with an empty message, so nothing more is printed.
pub(crate) fn load_and_check(path: &Path) -> Result<(Project, croft_analysis::Program), String> {
    let project = Project::load(path)?;

    match project.check() {
        Ok(program) => Ok((project, program)),
        Err(diagnostics) => {
            print_diagnostics(&project, &diagnostics);
            Err(String::new())
        }
    }
}

/// `croft check`: checks a project and says how many files and contexts it has.
fn check(path: &Path) -> Result<(), String> {
    let (project, program) = load_and_check(path)?;

    let contexts = program
        .contexts
        .values()
        .filter(|c| !c.is_domain && !c.is_standard)
        .count();

    let files = project.files.len();

    let domain = program
        .domains
        .first()
        .map_or(String::new(), |d| format!("domain {} with ", d.name.name));

    println!(
        "Checked {files} file{}: {domain}{contexts} context{}, no errors",
        plural(files),
        plural(contexts)
    );

    Ok(())
}

/// The ending for a count of things: `s`, unless `count` is 1.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
