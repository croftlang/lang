//! The whole program, with every file's part of a context merged into one.
//!
//! A context may be split across files with `context Dungeon` headers, and an
//! aggregate with `aggregate Room` headers. Parts with the same name are joined
//! here, so the checks never need to know which file a declaration came from.

mod merge;

use std::collections::BTreeMap;

use croft_syntax::ast::*;

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
    /// The doc comment above the part that publishes, or else above the
    /// first part that has one.
    pub doc: Vec<String>,
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
    /// Part of the standard library, such as `Std.Files`: it needs no place
    /// in the domain's `contexts`, and no arrow to be used.
    pub is_standard: bool,
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

                // The part that publishes is the context's front door, so
                // its comment is the context's doc. Others describe a file.
                let publishes = (part.items.iter()).any(|i| matches!(i, ContextItem::Publish(_)));

                if !part.doc.is_empty() && (publishes || context.doc.is_empty()) {
                    context.doc = part.doc;
                }

                context.add_items(part.items);
            }
        }

        for context in program.contexts.values_mut() {
            context.mark_identity_fields();
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

impl Context {
    /// Marks the field that holds each aggregate's and entity's own identity:
    /// the one of the identity type declared `for` it.
    fn mark_identity_fields(&mut self) {
        for aggregate in &mut self.aggregates {
            let owner = aggregate.name.name.clone();

            if let Some(identity) = identity_for(&self.identities, &owner, None) {
                mark_fields(&mut aggregate.fields, identity);
            }

            for entity in &mut aggregate.entities {
                let name = entity.name.name.as_str();

                if let Some(identity) = identity_for(&self.identities, &owner, Some(name)) {
                    mark_fields(&mut entity.fields, identity);
                }
            }
        }
    }

    /// The identity declared for an aggregate, or for an entity inside it.
    pub fn identity_for(&self, aggregate: &str, entity: Option<&str>) -> Option<&IdentityDecl> {
        identity_for(&self.identities, aggregate, entity)
    }
}

fn identity_for<'a>(
    identities: &'a [IdentityDecl],
    aggregate: &str,
    entity: Option<&str>,
) -> Option<&'a IdentityDecl> {
    identities
        .iter()
        .find(|identity| identifies(&identity.owner, aggregate, entity))
}

/// Whether an identity's `for` names this aggregate or entity. An entity is
/// written by its own name or, to tell it apart, as `Room.Enemy`.
pub fn identifies(owner: &Path, aggregate: &str, entity: Option<&str>) -> bool {
    let names: Vec<&str> = owner.segments.iter().map(|s| s.name.as_str()).collect();

    match (entity, names.as_slice()) {
        (None, [name]) => *name == aggregate,
        (Some(entity), [name]) => *name == entity,
        (Some(entity), [outer, name]) => *outer == aggregate && *name == entity,
        _ => false,
    }
}

fn mark_fields(fields: &mut [Field], identity: &IdentityDecl) {
    for field in fields {
        let path = &field.ty.path.segments;
        field.is_identity = path.len() == 1 && path[0].name == identity.name.name;
    }
}
