//! Finding, reading and checking a project: its source files and its `croft.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use croft_analysis::{Manifest, Program, MANIFEST_FILE};
use croft_syntax::{parse, Diagnostic, SourceId};

pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

/// A project is a folder of source files (`.croft`, or `.cro` on machines with
/// 8.3 file names) with a `croft.toml`, or a single source file.
pub struct Project {
    pub folder: PathBuf,
    /// The source files, then `croft.toml` last when there is one.
    pub files: Vec<SourceFile>,
    has_manifest: bool,
}

impl Project {
    pub fn load(path: &Path) -> Result<Project, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("cannot open `{}`: {error}", path.display()))?;

        let folder = if metadata.is_file() {
            path.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .to_path_buf()
        } else {
            path.to_path_buf()
        };

        let paths = if metadata.is_file() {
            vec![path.to_path_buf()]
        } else {
            let mut paths = Vec::new();
            find_source_files(path, &mut paths)
                .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
            paths.sort();
            paths
        };

        if paths.is_empty() {
            return Err(format!(
                "no .croft or .cro files found in `{}`",
                path.display()
            ));
        }

        let mut files = Vec::new();

        for path in paths {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
            files.push(SourceFile { path, text });
        }

        let manifest_path = folder.join(MANIFEST_FILE);
        let has_manifest = metadata.is_dir() && manifest_path.is_file();

        if has_manifest {
            let text = fs::read_to_string(&manifest_path)
                .map_err(|error| format!("cannot read `{}`: {error}", manifest_path.display()))?;

            files.push(SourceFile {
                path: manifest_path,
                text,
            });
        }

        Ok(Project {
            folder,
            files,
            has_manifest,
        })
    }

    /// Parses and checks every file as one program.
    pub fn check(&self) -> Result<Program, Vec<Diagnostic>> {
        let mut modules = Vec::new();
        let mut errors = Vec::new();
        let mut manifest = None;

        for (index, file) in self.files.iter().enumerate() {
            let source = SourceId(index as u32);

            if self.has_manifest && index + 1 == self.files.len() {
                match Manifest::parse(&file.text, source) {
                    Ok(parsed) => manifest = Some(parsed),
                    Err(error) => errors.push(error),
                }

                continue;
            }

            match parse(&file.text, source) {
                Ok(module) => modules.push(module),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        croft_analysis::check(modules, manifest)
    }
}

fn find_source_files(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        if name.starts_with('.') || name == "target" {
            continue;
        }

        if path.is_dir() {
            find_source_files(&path, found)?;
        } else if croft_syntax::is_source_file(&path) {
            found.push(path);
        }
    }

    Ok(())
}
