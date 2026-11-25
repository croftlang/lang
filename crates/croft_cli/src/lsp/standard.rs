//! The standard library as files an editor can open.
//!
//! Its contexts are built into the compiler, so there is no file of theirs
//! in the project. Going to one of their declarations writes each of them
//! to a folder of their own, once, and opens it from there. The files are
//! read-only: changing them would change nothing.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use croft_analysis::standard::{standard_source_id, STANDARD_SOURCES};
use croft_syntax::SourceId;

/// The folder the standard contexts are written to, one for each version of Croft.
pub fn folder() -> PathBuf {
    std::env::temp_dir().join(format!("croft-{}-standard", env!("CARGO_PKG_VERSION")))
}

fn file_name(context: &str) -> String {
    format!("{context}.croft")
}

/// Writes the standard contexts, unless they already are, and gives back
/// the folder they are in, or `None` when it cannot be written.
fn written() -> Option<&'static Path> {
    static WRITTEN: OnceLock<Option<PathBuf>> = OnceLock::new();

    let folder = WRITTEN.get_or_init(|| {
        let folder = folder();
        std::fs::create_dir_all(&folder).ok()?;

        for (context, text) in STANDARD_SOURCES {
            let path = folder.join(file_name(context));

            if std::fs::read_to_string(&path).is_ok_and(|old| old == text) {
                continue;
            }

            // A read-only file from an earlier run is replaced, not written over.
            let _ = std::fs::remove_file(&path);

            std::fs::write(&path, text).ok()?;

            let mut permissions = std::fs::metadata(&path).ok()?.permissions();
            permissions.set_readonly(true);

            let _ = std::fs::set_permissions(&path, permissions);
        }

        Some(folder)
    });

    folder.as_deref()
}

/// The file a standard source can be read from, written if need be.
pub fn path_of(id: SourceId) -> Option<PathBuf> {
    let (context, _) = STANDARD_SOURCES
        .iter()
        .find(|(context, _)| standard_source_id(context) == id)?;

    Some(written()?.join(file_name(context)))
}

/// The standard source a path is one of, and its text.
pub fn source_at(path: &Path) -> Option<(SourceId, &'static str)> {
    if path.parent()? != folder() {
        return None;
    }

    let name = path.file_name()?.to_str()?;

    let (context, text) = STANDARD_SOURCES
        .iter()
        .find(|(context, _)| file_name(context) == name)?;

    Some((standard_source_id(context), text))
}
