//! The project file, `dn.toml`: the program's name, and for each target the
//! adapter that fills each port and the C files that hold its native
//! functions. The program runs its `domain`, unless a target names a service
//! to run instead with `main`.

mod raw;

use dn_syntax::ast::{Identifier, Path};
use dn_syntax::{Diagnostic, SourceId, Span};

pub const MANIFEST_FILE: &str = "dn.toml";

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: Identifier,
    pub version: Option<String>,
    pub targets: Vec<TargetConfig>,
}

/// One `[target.<name>]` table.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    pub name: Identifier,
    /// A service to run instead of the domain on this target, such as a test entry point.
    pub main: Option<Identifier>,
    /// C files, relative to the project folder, holding the native functions.
    pub native: Vec<Identifier>,
    /// Each port, and the adapter that fills it on this target.
    pub adapters: Vec<(Identifier, Identifier)>,
}

impl Manifest {
    pub fn parse(text: &str, source: SourceId) -> Result<Manifest, Diagnostic> {
        raw::parse(text, source)
    }

    pub fn target(&self, name: &str) -> Option<&TargetConfig> {
        self.targets.iter().find(|target| target.name.name == name)
    }
}

/// A name from `dn.toml` as a path: `Dungeon.Screen` has two segments.
pub fn to_path(name: &Identifier) -> Path {
    let segments = name.name.split('.').map(|segment| Identifier {
        name: segment.to_string(),
        span: name.span,
    });
    Path {
        segments: segments.collect(),
    }
}

/// The line and column of a byte offset into the file.
fn span_at(text: &str, offset: usize, source: SourceId) -> Span {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |rest| rest.chars().count()) as u32
        + 1;
    Span {
        source,
        line,
        column,
    }
}
