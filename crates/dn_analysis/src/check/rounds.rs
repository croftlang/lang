//! Checking a program in rounds, each adding the generic instances the one
//! before asked for, until a round asks for none that are new.

use std::cell::RefCell;

use dn_syntax::ast::File;
use dn_syntax::{Diagnostic, SourceId, Span};

use super::Checker;
use crate::facts::Facts;
use crate::generics::{instance_file, Request, MAX_ROUNDS};
use crate::manifest::Manifest;
use crate::program::Program;
use crate::symbols::build_table;

/// Merges the files into one program and checks it, with the project file if there is one.
/// Generic templates are checked through their instances, which are made in
/// rounds: each round checks the program with the instances made so far, and
/// the next adds those it asked for, until none are new.
pub fn check(files: Vec<File>, manifest: Option<Manifest>) -> Result<Program, Vec<Diagnostic>> {
    let mut files = files;
    let bound: Vec<String> = (manifest.iter().flat_map(|m| &m.targets))
        .flat_map(|t| &t.adapters)
        .map(|(_, adapter)| adapter.name.clone())
        .collect();
    crate::standard::add_standard_sources(&mut files, &bound);
    let mut made: Vec<Request> = Vec::new();
    for _ in 0..MAX_ROUNDS {
        let (result, requests) = check_round(files.clone(), manifest.clone());
        let new: Vec<Request> = requests.into_iter().filter(|r| !made.contains(r)).collect();
        if new.is_empty() {
            return result;
        }
        for request in new {
            let source = SourceId(INSTANCE_SOURCES - made.len() as u32);
            if let Some(file) = instance_file(&request, &files, source) {
                files.push(file);
            }
            made.push(request);
        }
    }
    let example = made.last().map(Request::name).unwrap_or_default();
    let message =
        format!("generic types and functions ask for instances without end, such as `{example}`");
    Err(vec![Diagnostic::new(message, Span::default())])
}

/// Instances get source ids counting down from here, below the standard contexts'.
const INSTANCE_SOURCES: u32 = u32::MAX - 64;

/// One round of checking, and the instances it asked for.
fn check_round(
    files: Vec<File>,
    manifest: Option<Manifest>,
) -> (Result<Program, Vec<Diagnostic>>, Vec<Request>) {
    let mut program = Program::from_files(files);
    for (name, context) in &mut program.contexts {
        context.is_standard = crate::standard::standard_source(name).is_some();
    }
    program.manifest = manifest;
    let mut errors = Vec::new();
    let tables = program
        .contexts
        .iter()
        .map(|(name, context)| (name.clone(), build_table(context, &mut errors)))
        .collect();
    let mut checker = Checker {
        program: &program,
        tables,
        errors,
        facts: Facts::default(),
        requests: RefCell::default(),
    };
    for context in program.contexts.values() {
        checker.check_context(context);
    }
    checker.check_domain();
    if let Some(manifest) = &program.manifest {
        checker.check_manifest(manifest);
    }
    if checker.errors.is_empty() {
        for context in program.contexts.values() {
            checker.check_bodies(context);
        }
    }
    let Checker {
        errors,
        facts,
        requests,
        ..
    } = checker;
    let requests = requests.into_inner();
    let mut errors = errors;
    if errors.is_empty() {
        program.facts = facts;
        return (Ok(program), requests);
    }
    errors.sort_by_key(|error| (error.span.source.0, error.span.line, error.span.column));
    (Err(errors), requests)
}
