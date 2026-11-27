//! `croft compile`: checks a project and writes C for one of its targets.

use std::fs;
use std::path::{Path, PathBuf};

use crate::load_and_check;

/// What `croft compile` wrote for one target.
pub struct Output {
    /// The package name from `croft.toml`, or the default's.
    pub name: String,
    pub target: String,
    /// `target/croft/<target>` in the project folder.
    pub folder: PathBuf,
    /// The C files the compiler wrote, by file name inside `folder`.
    pub sources: Vec<String>,
    /// The project's own native C files from `croft.toml`, copied into `folder`.
    pub native: Vec<String>,
}

/// `croft compile`: writes C for one target and says where it went.
pub fn compile(arguments: &[String]) -> Result<(), String> {
    let (path, target) = parse_arguments(arguments)?;
    let output = write_c(&path, target)?;

    println!(
        "Wrote C for target `{}` to {}",
        output.target,
        output.folder.display()
    );

    Ok(())
}

/// `croft size`: checks a project, then reports the memory its types and
/// functions take on one target.
pub fn size(arguments: &[String]) -> Result<(), String> {
    let (path, target) = parse_arguments(arguments)?;
    let (_, program) = load_and_check(&path)?;
    let target = choose_target(&program, target, &path)?;
    print!("{}", croft_codegen::size_report(&program, &target)?);

    Ok(())
}

/// Reads `[path] --target <name>` from the arguments.
pub fn parse_arguments(arguments: &[String]) -> Result<(PathBuf, Option<String>), String> {
    let mut path = None;
    let mut target = None;
    let mut rest = arguments.iter();

    while let Some(argument) = rest.next() {
        if argument == "--target" {
            let name = rest
                .next()
                .ok_or("`--target` needs a name, such as `--target desktop`")?;

            target = Some(name.clone());
        } else if let Some(name) = argument.strip_prefix("--target=") {
            target = Some(name.to_string());
        } else if argument.starts_with("--") {
            return Err(format!("unknown option `{argument}`"));
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            return Err(format!("unexpected argument `{argument}`"));
        }
    }

    Ok((path.unwrap_or_else(|| PathBuf::from(".")), target))
}

/// Checks the project at `path`, then writes its C for `target` to `target/croft/<target>`.
pub fn write_c(path: &Path, target: Option<String>) -> Result<Output, String> {
    let (project, program) = load_and_check(path)?;

    let target = choose_target(&program, target, &project.folder)?;
    let manifest = program.manifest.as_ref().ok_or("croft.toml is missing")?;

    let files = croft_codegen::generate(&program, &target)?;
    let folder = project.folder.join("target").join("croft").join(&target);
    fs::create_dir_all(&folder)
        .map_err(|error| format!("cannot create `{}`: {error}", folder.display()))?;

    let mut sources = Vec::new();

    for file in &files {
        write(&folder.join(&file.name), &file.contents)?;

        if file.name.ends_with(".c") {
            sources.push(file.name.clone());
        }
    }

    let mut native_sources = Vec::new();

    for native in manifest.target(&target).map_or(&[][..], |t| &t.native[..]) {
        let source = project.folder.join(&native.name);
        let name = Path::new(&native.name).file_name().unwrap_or_default();
        fs::copy(&source, folder.join(name))
            .map_err(|error| format!("cannot copy native file `{}`: {error}", source.display()))?;

        let name = name.to_string_lossy().into_owned();

        if name.ends_with(".c") {
            native_sources.push(name);
        }
    }

    Ok(Output {
        name: manifest.name.name.clone(),
        target,
        folder,
        sources,
        native: native_sources,
    })
}

/// Writes `contents` to the file at `path`.
fn write(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}

/// The target named with `--target`, or the only one when there is just one.
/// Fails when the project has several and none is named, listing them.
fn choose_target(
    program: &croft_analysis::Program,
    target: Option<String>,
    folder: &Path,
) -> Result<String, String> {
    let Some(manifest) = &program.manifest else {
        let folder = folder.display();

        return Err(format!(
            "`{folder}` has no croft.toml, which says what to run and on which targets"
        ));
    };

    if let Some(target) = target {
        return Ok(target);
    }

    let targets: Vec<&str> = manifest
        .targets
        .iter()
        .map(|t| t.name.name.as_str())
        .collect();

    let is_default = manifest.name.span.source == croft_analysis::DEFAULT_MANIFEST_SOURCE;

    match targets[..] {
        [only] => Ok(only.to_string()),
        [] if is_default => Err(
            "there is nothing to run: declare a `domain`, or add a croft.toml whose target names a service with `main`"
                .to_string(),
        ),
        [] => Err("croft.toml has no targets: add one, such as `[target.desktop]`".to_string()),
        _ => Err(format!(
            "choose a target with `--target`; croft.toml has: {}",
            targets.join(", ")
        )),
    }
}
