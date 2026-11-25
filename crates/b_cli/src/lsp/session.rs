//! What the server knows: the files open in the editor, and each project's
//! latest analysis.
//!
//! A project is the folder holding the nearest `b.toml`, or a `.b` file on
//! its own when there is none. It is checked again whenever one of its files
//! changes. A file that no longer parses keeps its last good syntax tree, so
//! hover and go to definition keep working while it is being edited.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use b_analysis::{Analysis, Manifest, MANIFEST_FILE};
use b_syntax::ast::File;
use b_syntax::{parse, Diagnostic, SourceId};

use crate::project::Project;

/// A file as it was checked: its syntax tree came from `text`.
pub struct CheckedFile {
    pub path: PathBuf,
    pub id: SourceId,
    pub text: String,
}

pub struct ProjectState {
    pub analysis: Analysis,
    pub files: Vec<CheckedFile>,
    /// Syntax errors, then what checking found.
    pub diagnostics: Vec<Diagnostic>,
}

impl ProjectState {
    pub fn file(&self, id: SourceId) -> Option<&CheckedFile> {
        self.files.iter().find(|file| file.id == id)
    }

    pub fn file_at(&self, path: &Path) -> Option<&CheckedFile> {
        self.files.iter().find(|file| file.path == path)
    }

    /// The text of a source: one of the project's files, or the standard library.
    pub fn text(&self, id: SourceId) -> Option<&str> {
        if let Some(file) = self.file(id) {
            return Some(&file.text);
        }
        b_analysis::standard::STANDARD_SOURCES
            .iter()
            .find(|(context, _)| b_analysis::standard::standard_source_id(context) == id)
            .map(|(_, text)| *text)
    }
}

#[derive(Default)]
pub struct Session {
    /// The workspace folder: the search for `b.toml` stops here.
    pub root: Option<PathBuf>,
    /// The text of each file open in the editor, which may not be saved.
    open: HashMap<PathBuf, String>,
    /// Each file keeps one source id for the whole session, so a syntax tree
    /// kept from an earlier check still points at the right file.
    ids: HashMap<PathBuf, SourceId>,
    /// The last text of each file that parsed, and its tree.
    last_parsed: HashMap<PathBuf, (String, File)>,
    pub projects: HashMap<PathBuf, ProjectState>,
}

impl Session {
    pub fn new(root: Option<PathBuf>) -> Session {
        Session {
            root,
            ..Session::default()
        }
    }

    pub fn open(&mut self, path: PathBuf, text: String) {
        self.open.insert(path, text);
    }

    pub fn close(&mut self, path: &Path) {
        self.open.remove(path);
    }

    /// The project a file belongs to.
    pub fn project_for(&self, path: &Path) -> Option<&ProjectState> {
        self.projects
            .values()
            .find(|project| project.file_at(path).is_some())
    }

    /// Checks the project `path` belongs to again, and returns its key.
    pub fn refresh(&mut self, path: &Path) -> Result<PathBuf, String> {
        let key = self.project_key(path);
        let state = self.check(&key, path)?;
        self.projects.insert(key.clone(), state);
        Ok(key)
    }

    /// The folder with the nearest `b.toml`, looking no higher than the
    /// workspace folder, or the file itself.
    fn project_key(&self, path: &Path) -> PathBuf {
        for folder in path.ancestors().skip(1) {
            if folder.join(MANIFEST_FILE).is_file() {
                return folder.to_path_buf();
            }
            if Some(folder) == self.root.as_deref() {
                break;
            }
        }
        path.to_path_buf()
    }

    fn id_for(&mut self, path: &Path) -> SourceId {
        let next = SourceId(self.ids.len() as u32);
        *self.ids.entry(path.to_path_buf()).or_insert(next)
    }

    fn check(&mut self, key: &Path, changed: &Path) -> Result<ProjectState, String> {
        let mut paths: Vec<PathBuf> = match Project::load(key) {
            Ok(project) => project.files.into_iter().map(|file| file.path).collect(),
            Err(_) if self.open.contains_key(changed) => Vec::new(),
            Err(message) => return Err(message),
        };
        // A new file that has not been saved yet.
        let is_source = changed.extension().is_some_and(|e| e == "b");
        if is_source && !paths.iter().any(|path| path == changed) {
            paths.insert(0, changed.to_path_buf());
        }
        let (mut sources, mut manifest) = (Vec::new(), None);
        let (mut files, mut diagnostics) = (Vec::new(), Vec::new());
        for path in paths {
            let text = match self.open.get(&path) {
                Some(text) => text.clone(),
                None => std::fs::read_to_string(&path).unwrap_or_default(),
            };
            let id = self.id_for(&path);
            if path.file_name().is_some_and(|name| name == MANIFEST_FILE) {
                match Manifest::parse(&text, id) {
                    Ok(parsed) => manifest = Some(parsed),
                    Err(error) => diagnostics.push(error),
                }
                files.push(CheckedFile { path, id, text });
                continue;
            }
            let checked_text = match parse(&text, id) {
                Ok(tree) => {
                    self.last_parsed
                        .insert(path.clone(), (text.clone(), tree.clone()));
                    sources.push(tree);
                    text
                }
                Err(error) => {
                    diagnostics.push(error);
                    match self.last_parsed.get(&path) {
                        Some((old_text, tree)) => {
                            sources.push(tree.clone());
                            old_text.clone()
                        }
                        None => text,
                    }
                }
            };
            files.push(CheckedFile {
                path,
                id,
                text: checked_text,
            });
        }
        let analysis = b_analysis::analyze(sources, manifest);
        diagnostics.extend(analysis.diagnostics.iter().cloned());
        Ok(ProjectState {
            analysis,
            files,
            diagnostics,
        })
    }

    /// The text a diagnostic should be placed against: what the editor shows,
    /// for a file that is open.
    pub fn current_text<'a>(&'a self, file: &'a CheckedFile) -> &'a str {
        self.open.get(&file.path).unwrap_or(&file.text)
    }
}
