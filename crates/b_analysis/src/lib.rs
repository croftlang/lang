//! Checking b programs: merging files, resolving names and checking declarations.

pub mod check;
pub mod facts;
pub mod generics;
pub mod manifest;
pub mod naming;
pub mod program;
pub mod standard;
pub mod symbols;
pub mod types;

pub use check::{analyze, check, Analysis, Model, Operation, VariantInfo};
pub use facts::{Facts, LocalName, Meaning, Reference};
pub use manifest::{Manifest, TargetConfig, MANIFEST_FILE};
pub use program::{Context, Program};
pub use types::{Type, MAX_PLACES, MAX_SHIFT};
