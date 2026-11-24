//! `dn compile`: checks a project and writes C for one target.

use std::fs;
use std::path::{Path, PathBuf};

use crate::load_and_check;

pub fn compile(arguments: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut target = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        if argument == "--target" {
            target = Some(
                rest.next()
                    .ok_or("`--target` needs a name, such as `--target desktop`")?
                    .clone(),
            );
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
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let (_, program) = load_and_check(&path)?;
    let Some(target) = target else {
        let targets = dn_codegen::targets(&program).join(", ");
        return Err(format!(
            "choose a target with `--target`; this program has: {targets}"
        ));
    };
    let files = dn_codegen::generate(&program, &target)?;
    let folder = project_folder(&path)
        .join("target")
        .join("dn")
        .join(&target);
    fs::create_dir_all(&folder)
        .map_err(|error| format!("cannot create `{}`: {error}", folder.display()))?;
    for file in &files {
        let file_path = folder.join(&file.name);
        fs::write(&file_path, &file.contents)
            .map_err(|error| format!("cannot write `{}`: {error}", file_path.display()))?;
    }
    println!("Wrote C for target `{target}` to {}", folder.display());
    Ok(())
}

/// The folder a project lives in: the path itself, or a single file's folder.
fn project_folder(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.to_path_buf();
    }
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}
