//! `dn compile`: checks a project and writes C for one target from its `dn.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::load_and_check;

pub fn compile(arguments: &[String]) -> Result<(), String> {
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
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let (project, program) = load_and_check(&path)?;
    let Some(manifest) = &program.manifest else {
        let folder = project.folder.display();
        return Err(format!(
            "`{folder}` has no dn.toml, which says what to run and on which targets"
        ));
    };
    let Some(target) = target else {
        let targets: Vec<&str> = manifest
            .targets
            .iter()
            .map(|t| t.name.name.as_str())
            .collect();
        return Err(format!(
            "choose a target with `--target`; dn.toml has: {}",
            targets.join(", ")
        ));
    };
    let files = dn_codegen::generate(&program, &target)?;
    let folder = project.folder.join("target").join("dn").join(&target);
    fs::create_dir_all(&folder)
        .map_err(|error| format!("cannot create `{}`: {error}", folder.display()))?;
    for file in &files {
        write(&folder.join(&file.name), &file.contents)?;
    }
    for native in manifest.target(&target).map_or(&[][..], |t| &t.native[..]) {
        let source = project.folder.join(&native.name);
        let name = Path::new(&native.name).file_name().unwrap_or_default();
        fs::copy(&source, folder.join(name))
            .map_err(|error| format!("cannot copy native file `{}`: {error}", source.display()))?;
    }
    println!("Wrote C for target `{target}` to {}", folder.display());
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}
