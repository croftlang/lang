//! Declaration checks: every name resolves, and every declaration has the
//! parts its kind needs. Bodies are checked in a later pass.

mod adapter;
mod aggregate;
mod assign;
mod behaviour;
mod body;
mod boundary;
mod calls;
mod constraints;
mod data;
mod declared;
mod domain;
mod evaluate;
mod find;
mod generics;
mod identity;
mod known;
mod manifest;
mod model;
mod model_types;
mod names;
mod numbers;
mod operations;
mod ports;
mod publish;
mod repository;
mod resolve;
mod rounds;
#[cfg(test)]
mod tests;
mod type_arguments;
mod types;
mod variant;

use std::cell::RefCell;
use std::collections::HashMap;

use croft_syntax::{Diagnostic, Span};

use crate::facts::{Facts, Meaning};
use crate::generics::Request;
use crate::program::{Context, Program};
use crate::symbols::SymbolTable;

pub use model::{Model, VariantInfo};
pub use operations::Operation;
pub use repository::StorageShape;
pub(crate) use resolve::{Resolved, Scope};

pub use rounds::{analyze, check, Analysis};

/// Checks a program's declarations and bodies, collecting errors and what it learns.
pub(crate) struct Checker<'a> {
    program: &'a Program,
    tables: HashMap<String, SymbolTable>,
    errors: Vec<Diagnostic>,
    facts: Facts,
    /// Instances of generic templates this round needs but doesn't have yet.
    requests: RefCell<Vec<Request>>,
    /// Names resolved so far, with where each was written. `resolve` takes
    /// `&self`, so they are kept here until checking ends.
    references: RefCell<Vec<crate::facts::Reference>>,
    /// Arithmetic with no 32-bit answer, found in bodies and through calls.
    faults: RefCell<evaluate::Faults>,
}

impl<'a> Checker<'a> {
    /// Records what an expression refers to, for code generation.
    fn record(&mut self, id: croft_syntax::ast::ExprId, meaning: Meaning) {
        self.facts.meanings.insert(id, meaning);
    }

    /// Reports an error at `span`.
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(Diagnostic::new(message, span));
    }

    /// Checks every declaration in a context. Generic templates are checked
    /// only through their instances.
    fn check_context(&mut self, context: &'a Context) {
        let scope = Scope {
            context,
            aggregate: None,
        };

        self.check_names(context);
        self.check_publish(context);
        self.check_uses(context);

        for value in &context.values {
            if value.type_parameters.is_empty() {
                self.check_value(&scope, value);
            } else {
                self.check_type_parameters(&value.type_parameters);
            }
        }

        for enum_decl in &context.enums {
            self.check_enum(&scope, enum_decl);
        }

        for error in &context.errors {
            self.check_fields(&scope, &error.fields);
        }

        for constant in &context.consts {
            self.check_type(&scope, &constant.ty, false);
        }

        for identity in &context.identities {
            self.check_identity(&scope, identity);
        }

        for constraint in &context.constraints {
            self.check_constraint(&scope, constraint);
        }

        for aggregate in &context.aggregates {
            self.check_aggregate(context, aggregate);
        }

        for function in &context.functions {
            if function.type_parameters.is_empty() {
                self.check_function(&scope, function);
            } else {
                self.check_type_parameters(&function.type_parameters);
            }
        }

        for policy in &context.policies {
            self.check_policy(&scope, policy);
        }

        for port in &context.ports {
            self.check_port(&scope, port);
        }

        for adapter in &context.adapters {
            self.check_adapter(&scope, adapter);
        }

        for repository in &context.repositories {
            self.check_repository(&scope, repository);
        }
    }
}
