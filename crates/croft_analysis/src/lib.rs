//! Checking Croft programs: merging files, resolving names and checking declarations.

pub mod builtin_members;
pub mod builtins;
pub mod check;
pub mod facts;
pub mod generics;
pub mod manifest;
pub mod naming;
pub mod program;
pub mod standard;
pub mod symbols;
pub mod types;

pub use builtin_members::built_in_members;
pub use builtins::built_in_doc;
pub use check::{analyze, check, Analysis, Model, Operation, StorageShape, VariantInfo};
pub use facts::{BuiltinName, Facts, LocalName, Meaning, Member, MemberKind, Reference};
pub use manifest::{Manifest, TargetConfig, MANIFEST_FILE};
pub use program::{Context, Program};
pub use types::{Type, MAX_PLACES, MAX_SHIFT};
