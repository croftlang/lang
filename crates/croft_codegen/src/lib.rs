//! Generates portable C89 from a checked Croft program, for one target.

/// Writing the C for function bodies.
mod function;
/// The state shared by everything generated for one program.
mod generator;
/// C names for Croft declarations and types.
mod names;
/// The parts of the generated program outside function bodies.
mod program;
/// The memory size report.
mod report;
/// C type definitions and sizes.
mod types;

use croft_analysis::Program;

pub use generator::Generator;
pub use report::size_report;

/// One generated file: its name and its contents.
pub struct GeneratedFile {
    pub name: String,
    pub contents: String,
}

/// Generates the C files for a program on one target, such as `desktop` or `spectrum_48k`.
pub fn generate(program: &Program, target: &str) -> Result<Vec<GeneratedFile>, String> {
    Generator::new(program, target)?.generate()
}
