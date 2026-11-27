//! A read-only view of a checked program's declarations, for code generation.

use super::operations::Operation;
use super::repository::StorageShape;
use std::collections::HashMap;

use croft_syntax::ast::{Field, Path, TypeExpr};

use super::{Checker, Scope};
use crate::facts::Facts;
use crate::program::Program;
use crate::symbols::build_table;
use crate::types::{Resolved, Type};

/// One variant of an enum or error set, with its fields' types.
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    /// The error the variant stands for, when it is one.
    pub error: Option<Resolved>,
    /// The enum or error that declares it, which may be one this enum includes.
    pub declared_in: Resolved,
}

/// A checked program's declarations, for code generation to ask about.
pub struct Model<'a> {
    pub(super) checker: Checker<'a>,
}

impl<'a> Model<'a> {
    /// The view of a checked program.
    pub fn new(program: &'a Program) -> Model<'a> {
        let mut errors = Vec::new();

        let tables: HashMap<_, _> = program
            .contexts
            .iter()
            .map(|(name, context)| (name.clone(), build_table(context, &mut errors)))
            .collect();

        Model {
            checker: Checker {
                program,
                tables,
                errors,
                facts: Facts::default(),
                requests: std::cell::RefCell::default(),
                references: std::cell::RefCell::default(),
            },
        }
    }

    /// Where names are looked up: a context, and optionally one of its aggregates.
    fn scope(&self, context: &str, aggregate: Option<&str>) -> Scope<'a> {
        let context = &self.checker.program.contexts[context];
        let aggregate =
            aggregate.and_then(|name| context.aggregates.iter().find(|a| a.name.name == name));

        Scope {
            context,
            aggregate: aggregate.map(|a| a.name.name.as_str()),
        }
    }

    /// The type a written type stands for, looked up from `context` and `aggregate`.
    pub fn lower_type(&self, context: &str, aggregate: Option<&str>, ty: &TypeExpr) -> Type {
        self.checker.lower_type(&self.scope(context, aggregate), ty)
    }

    /// What a path names, looked up from `context` and `aggregate`.
    pub fn resolve(&self, context: &str, aggregate: Option<&str>, path: &Path) -> Option<Resolved> {
        self.checker.resolve(&self.scope(context, aggregate), path)
    }

    /// The declaration `name` in `context`, if there is one.
    pub fn lookup(&self, context: &str, name: &str) -> Option<Resolved> {
        self.checker.lookup(context, name)
    }

    /// The fields of a value, entity, aggregate, error or event, with their types.
    pub fn fields(&self, named: &Resolved) -> Vec<(String, Type, &'a Field)> {
        let scope = self.checker.scope_of(named);
        let fields = self.checker.fields_of(named).unwrap_or(&[]);

        fields
            .iter()
            .map(|f| {
                (
                    f.name.name.clone(),
                    self.checker.lower_type(&scope, &f.ty),
                    f,
                )
            })
            .collect()
    }

    /// The type a single-value type holds, such as `Int` for `Score`.
    pub fn single_value_inner(&self, ty: &Type) -> Option<Type> {
        self.checker.single_value_inner(ty)
    }

    /// What an adapter for `port`, a port or a repository, provides.
    pub fn operations_of(&self, port: &Resolved) -> Vec<Operation<'a>> {
        self.checker.operations_of(port)
    }

    /// The type of an aggregate's identity field.
    pub fn identity_of(&self, aggregate: &Resolved) -> Type {
        self.checker.identity_of(aggregate)
    }

    /// The repository that stores an aggregate, if its context has one.
    pub fn repository_for(&self, aggregate: &Resolved) -> Option<Resolved> {
        self.checker.repository_for(aggregate)
    }

    /// The aggregate a repository stores.
    pub fn repository_aggregate(&self, repository: &Resolved) -> Option<Resolved> {
        self.checker.repository_aggregate(repository)
    }

    /// Which shape a repository's operation has, if `Storage` can provide it.
    pub fn storage_shape(&self, repository: &Resolved, operation: &str) -> Option<StorageShape> {
        let aggregate = self.checker.repository_aggregate(repository)?;
        let operations = self.checker.operations_of(repository);
        let operation = operations.iter().find(|op| op.name == operation)?;
        self.checker.storage_shape(&aggregate, operation)
    }

    /// The find and save `issue ... on Room(id)` loads and saves through.
    pub fn find_and_save(&self, repository: &Resolved) -> Option<(Operation<'a>, Operation<'a>)> {
        self.checker.find_and_save(repository)
    }
}
