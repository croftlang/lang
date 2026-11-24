//! Checking dn programs: merging files, resolving names and checking declarations.

pub mod check;
pub mod naming;
pub mod program;
pub mod symbols;

pub use check::check;
pub use program::{Context, Program};
