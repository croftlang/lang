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
    /// Every `domain` declared. A program has one; more are reported.
    pub domains: Vec<DomainDecl>,
    /// The project file, when there is one.
    pub manifest: Option<crate::manifest::Manifest>,
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
    /// Made from the `domain`: its `use` lines, and one service, its body.
    pub is_domain: bool,
}

impl Program {
    pub fn from_files(files: Vec<File>) -> Program {
        let mut program = Program::default();
        for file in files {
            program.domains.extend(file.domains);
            for part in file.contexts {
                let name = part.name.name.clone();
                let context = program
                    .contexts
                    .entry(name)
                    .or_insert_with(|| Context::new(part.name.clone()));
                context.add_items(part.items);
            }
        }
        if let Some(domain) = program.domains.first() {
            if !program.contexts.contains_key(&domain.name.name) {
                let context = Context::from_domain(domain);
                program.contexts.insert(domain.name.name.clone(), context);
            }
        }
        program
    }
}
