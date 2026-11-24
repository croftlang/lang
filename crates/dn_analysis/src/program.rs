//! The whole program, with every file's part of a context merged into one.
//!
//! A context may be split across files with `context Dungeon` headers, and an
//! aggregate with `aggregate Room` headers. Parts with the same name are joined
//! here, so the checks never need to know which file a declaration came from.

mod merge;

use std::collections::BTreeMap;

use dn_syntax::ast::*;

#[derive(Debug, Default)]
pub struct Program {
    pub contexts: BTreeMap<String, Context>,
    pub applications: Vec<ApplicationDecl>,
    /// Types and meanings of expressions, filled in by checking.
    pub facts: crate::facts::Facts,
}

#[derive(Debug)]
pub struct Context {
    pub name: Identifier,
    pub publish: Vec<Path>,
    pub uses: Vec<UseDecl>,
    pub values: Vec<ValueDecl>,
    pub enums: Vec<EnumDecl>,
    pub errors: Vec<RecordDecl>,
    pub consts: Vec<ConstDecl>,
    pub identities: Vec<IdentityDecl>,
    pub constraints: Vec<ConstraintDecl>,
    pub aggregates: Vec<AggregateDecl>,
    pub functions: Vec<FunctionDecl>,
    pub policies: Vec<PolicyDecl>,
    pub ports: Vec<PortDecl>,
    pub adapters: Vec<AdapterDecl>,
    pub repositories: Vec<RepositoryDecl>,
}

impl Program {
    pub fn from_files(files: Vec<File>) -> Program {
        let mut program = Program::default();
        for item in files.into_iter().flat_map(|file| file.items) {
            match item {
                TopItem::Application(application) => program.applications.push(application),
                TopItem::Context(part) => {
                    let name = part.name.name.clone();
                    let context = program
                        .contexts
                        .entry(name)
                        .or_insert_with(|| Context::new(part.name.clone()));
                    context.add_items(part.items);
                }
            }
        }
        program
    }
}
