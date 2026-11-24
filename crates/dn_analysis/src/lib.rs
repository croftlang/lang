//! Checking dn programs: merging files, resolving names and checking declarations.

pub mod check;
pub mod facts;
pub mod manifest;
pub mod naming;
pub mod program;
pub mod symbols;
pub mod types;

pub use check::{check, Model, VariantInfo};
pub use facts::{Facts, Meaning};
pub use manifest::{Manifest, TargetConfig, MANIFEST_FILE};
pub use program::{Context, Program};
pub use types::Type;
