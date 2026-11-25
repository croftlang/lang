//! Reading Croft source code: tokens, syntax tree and parser.

pub mod ast;
pub mod keyword;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use parser::parse;
pub use span::{Diagnostic, SourceId, Span};

/// The extensions of Croft source files: `.croft`, and `.cro` for machines
/// whose file names are limited to 8.3.
pub const SOURCE_EXTENSIONS: [&str; 2] = ["croft", "cro"];

/// Whether a path names a Croft source file. 8.3 file systems often give
/// names back in capitals, so `ROOM.CRO` counts too.
pub fn is_source_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SOURCE_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

#[cfg(test)]
mod source_file_tests {
    use super::is_source_file;
    use std::path::Path;

    #[test]
    fn knows_both_extensions_in_any_case() {
        for name in ["cargo.croft", "desktop.cro", "DESKTOP.CRO", "Cargo.Croft"] {
            assert!(is_source_file(Path::new(name)), "{name}");
        }

        for name in ["cargo.b", "croft.toml", "cargo.crofts", "croft"] {
            assert!(!is_source_file(Path::new(name)), "{name}");
        }
    }
}
