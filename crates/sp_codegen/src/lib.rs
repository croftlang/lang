//! Turning checked sp programs into Rust.

pub mod cargo;
pub mod rust;
mod rust_names;
mod source_text;

pub use cargo::{cargo_manifest, crate_name};
pub use rust::generate_rust;
