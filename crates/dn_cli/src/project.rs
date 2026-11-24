//! Finding, reading and checking the `.dn` files of a project.

use std::fs;
use std::path::{Path, PathBuf};

use dn_analysis::Program;
use dn_syntax::{parse, Diagnostic, SourceId};

pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

/// A project is a folder of `.dn` files, or a single `.dn` file.
pub struct Project {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

impl Project {
    pub fn load(path: &Path) -> Result<Project, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("cannot open `{}`: {error}", path.display()))?;
        let (root, paths) = if metadata.is_file() {
            let root = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .to_path_buf();
            (root, vec![path.to_path_buf()])
        } else {
            let mut paths = Vec::new();
            find_source_files(path, &mut paths)
                .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
            paths.sort();
            (path.to_path_buf(), paths)
        };
        if paths.is_empty() {
            return Err(format!("no .dn files found in `{}`", path.display()));
        }
        let mut files = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
            files.push(SourceFile { path, text });
        }
        Ok(Project { root, files })
    }

    /// Parses and checks every file as one program.
    pub fn check(&self) -> Result<Program, Vec<Diagnostic>> {
        let mut modules = Vec::new();
        let mut errors = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            match parse(&file.text, SourceId(index as u32)) {
                Ok(module) => modules.push(module),
                Err(error) => errors.push(error),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        dn_analysis::check(modules)
    }

    /// The name of the project folder, used for the generated crate.
    pub fn name(&self) -> String {
        let root = fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        root.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dn_project".to_string())
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("target").join("dn")
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
        } else if path.extension().is_some_and(|extension| extension == "dn") {
            found.push(path);
        }
    }
    Ok(())
}
