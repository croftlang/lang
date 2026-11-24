//! Generates portable C89 from a checked dn program, for one target.

mod function;
mod generator;
mod names;
mod program;
mod types;

use dn_analysis::Program;

pub use generator::Generator;

/// One generated file: its name and its contents.
pub struct GeneratedFile {
    pub name: String,
    pub contents: String,
}

/// Generates the C files for a program on one target, such as `desktop` or `spectrum_48k`.
pub fn generate(program: &Program, target: &str) -> Result<Vec<GeneratedFile>, String> {
    Generator::new(program, target)?.generate()
}

/// The targets an application declares, for error messages and `--target` help.
pub fn targets(program: &Program) -> Vec<String> {
    let targets = program
        .applications
        .iter()
        .flat_map(|application| &application.targets);
    targets.map(|target| target.name.name.clone()).collect()
}
