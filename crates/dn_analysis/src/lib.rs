//! Checking dn programs: merging files, resolving names and checking declarations.

pub mod check;
pub mod facts;
pub mod naming;
pub mod program;
pub mod symbols;
pub mod types;

pub use check::check;
pub use facts::{Facts, Meaning};
pub use program::{Context, Program};
pub use types::Type;
