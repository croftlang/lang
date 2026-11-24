//! The `dn` command-line tool.

mod diagnostics;
mod project;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use dn_codegen::{cargo_manifest, crate_name, generate_rust};

use crate::diagnostics::print_diagnostics;
use crate::project::Project;

const USAGE: &str = "\
dn - a domain language that compiles through Rust

Usage:
    dn check [path]               Check the .dn files in a folder (default: .)
    dn compile [path] [options]   Check, generate Rust and build it with cargo

Options for compile:
    --release    Build with optimisations
    --no-build   Generate the Rust crate without running cargo

    dn help       Show this message
    dn version    Show the version
";

struct CompileOptions {
    release: bool,
    build: bool,
}

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
        "compile" => parse_path(rest).and_then(|(path, flags)| {
            let options = compile_options(&flags)?;
            compile(&path, &options)
        }),
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("dn {}", env!("CARGO_PKG_VERSION"));
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

fn compile_options(flags: &[String]) -> Result<CompileOptions, String> {
    let mut options = CompileOptions {
        release: false,
        build: true,
    };
    for flag in flags {
        match flag.as_str() {
            "--release" => options.release = true,
            "--no-build" => options.build = false,
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    Ok(options)
}

fn load_and_check(path: &Path) -> Result<(Project, dn_analysis::Program), String> {
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
    println!(
        "Checked {} file{}, {} declaration{}: no errors",
        project.files.len(),
        plural(project.files.len()),
        program.items.len(),
        plural(program.items.len()),
    );
    Ok(())
}

fn compile(path: &Path, options: &CompileOptions) -> Result<(), String> {
    let (project, program) = load_and_check(path)?;
    let output_dir = project.output_dir();
    let name = crate_name(&project.name());
    write_if_changed(&output_dir.join("Cargo.toml"), &cargo_manifest(&name))?;
    write_if_changed(
        &output_dir.join("src").join("lib.rs"),
        &generate_rust(&program),
    )?;
    println!(
        "Generated crate `{name}` from {} file{} in {}",
        project.files.len(),
        plural(project.files.len()),
        output_dir.display()
    );
    if !options.build {
        return Ok(());
    }
    let mut cargo = Command::new("cargo");
    cargo
        .arg("build")
        .arg("--manifest-path")
        .arg(output_dir.join("Cargo.toml"));
    if options.release {
        cargo.arg("--release");
    }
    let status = cargo
        .status()
        .map_err(|error| format!("could not run cargo: {error}. Is Rust installed?"))?;
    if status.success() {
        println!("Compiled `{name}`");
        Ok(())
    } else {
        Err("cargo build failed on the generated code".to_string())
    }
}

/// Leaves unchanged files alone so cargo does not rebuild needlessly.
fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
