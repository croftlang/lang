//! The project file, `croft.toml`: the program's name, and for each target the
//! adapter that fills each port and the C files that hold its native
//! functions. The program runs its `domain`, unless a target names a service
//! to run instead with `main`.

mod raw;

use croft_syntax::ast::{File, Identifier, Path};

use crate::naming::to_snake_case;
use croft_syntax::{Diagnostic, SourceId, Span};

/// The project file's name.
pub const MANIFEST_FILE: &str = "croft.toml";

/// The target a project without a `croft.toml` is built for.
pub const DEFAULT_TARGET: &str = "desktop";

/// How many values a `Bytes` with no `Length` holds when the target doesn't
/// set `bytes_length`.
pub const DEFAULT_BYTES_LENGTH: u16 = 64;

/// How many characters a string with no `Length` holds when the target
/// doesn't set `strings_length`.
pub const DEFAULT_STRINGS_LENGTH: u16 = 32;

/// Where the names of the default project file are said to be written. It is
/// no file's id: project files count up from 0 and standard contexts count
/// down from the top, so errors about the default can be told apart.
pub const DEFAULT_MANIFEST_SOURCE: SourceId = SourceId(u32::MAX / 2);

/// The project file: the program's name and version, and its targets.
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
    /// Whether the program checks, while it runs, the constraints the compiler
    /// could not work out, and stops when one is broken. On unless the target
    /// sets `checks = false`.
    pub checks: bool,
    /// How many values a `Bytes` with no `Length` of its own holds on this
    /// target: `bytes_length`, or `DEFAULT_BYTES_LENGTH`.
    pub bytes_length: u16,
    /// How many characters a string with no `Length` of its own holds on
    /// this target: `strings_length`, or `DEFAULT_STRINGS_LENGTH`.
    pub strings_length: u16,
}

impl Manifest {
    /// Reads `croft.toml`, reporting the first error and where it is.
    pub fn parse(text: &str, source: SourceId) -> Result<Manifest, Diagnostic> {
        raw::parse(text, source)
    }

    /// The target named `name`, if the file has one.
    pub fn target(&self, name: &str) -> Option<&TargetConfig> {
        self.targets.iter().find(|target| target.name.name == name)
    }

    /// What a project without a `croft.toml` is given: one `desktop` target
    /// that runs the domain, with checks on and no adapters, named after the
    /// domain in snake_case. Without a domain there is nothing to run, so it
    /// has no targets and is named `fallback`; the project can still be checked.
    pub fn default_for(files: &[File], fallback: &str) -> Manifest {
        let named = |name: &str| Identifier {
            name: name.to_string(),
            span: Span {
                source: DEFAULT_MANIFEST_SOURCE,
                line: 1,
                column: 1,
            },
        };

        let Some(domain) = files.iter().flat_map(|file| &file.domains).next() else {
            return Manifest {
                name: named(fallback),
                version: Some("0.1.0".to_string()),
                targets: Vec::new(),
            };
        };

        Manifest {
            name: named(&to_snake_case(&domain.name.name)),
            version: Some("0.1.0".to_string()),
            targets: vec![TargetConfig {
                name: named(DEFAULT_TARGET),
                main: None,
                native: Vec::new(),
                adapters: Vec::new(),
                checks: true,
                bytes_length: DEFAULT_BYTES_LENGTH,
                strings_length: DEFAULT_STRINGS_LENGTH,
            }],
        }
    }
}

/// A name from `croft.toml` as a path: `Dungeon.Screen` has two segments.
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
