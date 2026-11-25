//! The `b` command-line tool.

mod compile;
mod diagnostics;
mod project;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::diagnostics::print_diagnostics;
use crate::project::Project;

const USAGE: &str = "\
b - a domain language that compiles to portable C

Usage:
    b check [path]      Check the .b files in a folder (default: .)
    b compile [path] --target <name>
                        Check, then write C for one target to target/b/<name>
    b help              Show this message
    b version           Show the version
";

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
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("b {}", env!("CARGO_PKG_VERSION"));
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

fn reject_flags(flags: &[String]) -> Result<(), String> {
    match flags.first() {
        Some(flag) => Err(format!("unknown option `{flag}`")),
        None => Ok(()),
    }
}

pub(crate) fn load_and_check(path: &Path) -> Result<(Project, b_analysis::Program), String> {
    let project = Project::load(path)?;
    match project.check() {
        Ok(program) => Ok((project, program)),
        Err(diagnostics) => {
            print_diagnostics(&project, &diagnostics);
            Err(String::new())
        }
    }
}

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

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
