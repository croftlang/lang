//! Checking sp programs: name resolution, type checking and naming rules.

pub mod checker;
pub mod naming;
pub mod program;
pub mod types;

pub use checker::check;
pub use program::{Program, Scope, OUTPUT_NAME};
pub use types::Type;
