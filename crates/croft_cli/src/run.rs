//! `croft run`: compiles a project for one target, builds the C with the system's
//! C compiler, and runs the program.

use std::env;
use std::process::Command;

use crate::compile;

/// Runs the program and returns its exit code. Arguments after `--` go to the program.
pub fn run(arguments: &[String]) -> Result<i32, String> {
    let (ours, theirs) = match arguments.iter().position(|a| a == "--") {
        Some(split) => (&arguments[..split], &arguments[split + 1..]),
        None => (arguments, &[][..]),
    };

    let (path, target) = compile::parse_arguments(ours)?;
    let output = compile::write_c(&path, target)?;

    let file_name = format!("{}{}", output.name, env::consts::EXE_SUFFIX);
    let program = output.folder.join(&file_name);
    // `CC` picks the compiler, as it does for make; `cc` is the usual name for it.
    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let mut build = Command::new(&compiler);

    build
        .current_dir(&output.folder)
        .arg("-std=c89")
        .args(&output.sources)
        .arg("-o")
        .arg(&file_name);

    // A standard adapter's C can need a library, as `CurlHttp` needs libcurl.
    for name in &output.sources {
        build.args(croft_std::links_for(name));
    }

    let built = build
        .status()
        .map_err(|error| format!("cannot run the C compiler `{compiler}`: {error}"))?;

    if !built.success() {
        return Err(format!(
            "the C compiler `{compiler}` could not build the C in {}",
            output.folder.display()
        ));
    }

    let ran = Command::new(&program)
        .args(theirs)
        .status()
        .map_err(|error| format!("cannot run `{}`: {error}", program.display()))?;

    // A program ended by a signal has no exit code; report it as a failure.
    Ok(ran.code().unwrap_or(1))
}
