//! Checking a program in rounds, each adding the generic instances the one
//! before asked for, until a round asks for none that are new.

use std::cell::RefCell;
use std::collections::HashMap;

use croft_syntax::ast::File;
use croft_syntax::{Diagnostic, SourceId, Span};

use super::Checker;
use crate::facts::Facts;
use crate::generics::{instance_file, Request, MAX_ROUNDS};
use crate::manifest::Manifest;
use crate::program::Program;
use crate::symbols::{build_table, SymbolTable};

/// Merges the files into one program and checks it, with the project file if there is one.
pub fn check(files: Vec<File>, manifest: Option<Manifest>) -> Result<Program, Vec<Diagnostic>> {
    let analysis = analyze(files, manifest);
    if analysis.diagnostics.is_empty() {
        Ok(analysis.program)
    } else {
        Err(analysis.diagnostics)
    }
}

/// What checking found, kept even when there are errors, for editors: the
/// program with what was learned about it, the errors, and each context's names.
pub struct Analysis {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: HashMap<String, SymbolTable>,
}

/// Checks the program like `check`, but keeps everything checking learned.
/// Generic templates are checked through their instances, which are made in
/// rounds: each round checks the program with the instances made so far, and
/// the next adds those it asked for, until none are new.
pub fn analyze(files: Vec<File>, manifest: Option<Manifest>) -> Analysis {
    let mut files = files;
    let bound: Vec<String> = (manifest.iter().flat_map(|m| &m.targets))
        .flat_map(|t| &t.adapters)
        .map(|(_, adapter)| adapter.name.clone())
        .collect();
    crate::standard::add_standard_sources(&mut files, &bound);
    let mut made: Vec<Request> = Vec::new();
    let mut last = None;
    for _ in 0..MAX_ROUNDS {
        let (analysis, requests) = check_round(files.clone(), manifest.clone());
        let new: Vec<Request> = requests.into_iter().filter(|r| !made.contains(r)).collect();
        if new.is_empty() {
            return analysis;
        }
        for request in new {
            let source = SourceId(INSTANCE_SOURCES - made.len() as u32);
            if let Some(file) = instance_file(&request, &files, source) {
                files.push(file);
            }
            made.push(request);
        }
        last = Some(analysis);
    }
    let example = made.last().map(Request::name).unwrap_or_default();
    let message =
        format!("generic types and functions ask for instances without end, such as `{example}`");
    let mut analysis = last.expect("at least one round");
    analysis.diagnostics = vec![Diagnostic::new(message, Span::default())];
    analysis
}

/// Instances get source ids counting down from here, below the standard contexts'.
const INSTANCE_SOURCES: u32 = u32::MAX - 64;

/// One round of checking, and the instances it asked for.
fn check_round(files: Vec<File>, manifest: Option<Manifest>) -> (Analysis, Vec<Request>) {
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
        references: RefCell::default(),
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
        mut facts,
        requests,
        references,
        tables,
        ..
    } = checker;
    let mut seen = std::collections::HashSet::new();
    let mut references = references.into_inner();
    references
        .retain(|r| seen.insert((r.span.source.0, r.span.line, r.span.column, r.name.clone())));
    facts.references = references;
    let mut seen = std::collections::HashSet::new();
    (facts.members)
        .retain(|m| seen.insert((m.span.source.0, m.span.line, m.span.column, m.name.clone())));
    program.facts = facts;
    let mut diagnostics = errors;
    diagnostics.sort_by_key(|error| (error.span.source.0, error.span.line, error.span.column));
    let analysis = Analysis {
        program,
        diagnostics,
        symbols: tables,
    };
    (analysis, requests.into_inner())
}
