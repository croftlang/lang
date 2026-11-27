//! `croft fmt`: lays source files out in the one Croft style.

use std::fs;
use std::path::{Path, PathBuf};

use croft_format::format_source;

use crate::project::source_paths;

/// Formats every source file at a path, or with `--check` only lists the
/// files that are not formatted and fails if there are any.
pub fn format(arguments: &[String]) -> Result<(), String> {
    let (path, flags) = crate::parse_path(arguments)?;
    let mut only_check = false;

    for flag in &flags {
        match flag.as_str() {
            "--check" => only_check = true,
            other => return Err(format!("unknown option `{other}`")),
        }
    }

    let paths = source_paths(&path)?;

    if paths.is_empty() {
        return Err(format!(
            "no .croft or .cro files found in `{}`",
            path.display()
        ));
    }

    let mut unformatted: Vec<PathBuf> = Vec::new();
    let mut failed = 0;

    for file in &paths {
        match format_file(file, only_check) {
            Ok(true) => unformatted.push(file.clone()),
            Ok(false) => {}
            Err(message) => {
                eprintln!("{message}");
                failed += 1;
            }
        }
    }

    report(&paths, &unformatted, only_check);

    if failed > 0 {
        eprintln!(
            "\n{failed} file{} could not be formatted",
            crate::plural(failed)
        );
        return Err(String::new());
    }

    if only_check && !unformatted.is_empty() {
        return Err(String::new());
    }

    Ok(())
}

/// Formats one file, writing it back unless `only_check`. Returns whether
/// it was not formatted already.
fn format_file(path: &Path, only_check: bool) -> Result<bool, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;

    let formatted = format_source(&text).map_err(|diagnostic| {
        format!(
            "{}:{}:{}: error: {}",
            path.display(),
            diagnostic.span.line,
            diagnostic.span.column,
            diagnostic.message
        )
    })?;

    if formatted == text {
        return Ok(false);
    }

    if !only_check {
        fs::write(path, formatted)
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
    }

    Ok(true)
}

/// Prints how many files were formatted or, with `--check`, which files
/// need formatting.
fn report(paths: &[PathBuf], unformatted: &[PathBuf], only_check: bool) {
    let files = paths.len();
    let changed = unformatted.len();

    if only_check {
        for path in unformatted {
            println!("{}", path.display());
        }

        if changed == 0 {
            println!(
                "Checked {files} file{}, all formatted",
                crate::plural(files)
            );
            return;
        }

        println!(
            "{changed} of {files} file{} need formatting",
            crate::plural(files)
        );
        return;
    }

    println!(
        "Formatted {changed} of {files} file{}",
        crate::plural(files)
    );
}
