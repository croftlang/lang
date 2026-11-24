//! Declaration checks: every name resolves, and every declaration has the
//! parts its kind needs. Bodies are checked in a later pass.

mod adapter;
mod aggregate;
mod application;
mod behaviour;
mod boundary;
mod data;
mod find;
mod names;
mod publish;
mod resolve;
mod rules;
#[cfg(test)]
mod tests;
mod types;

use std::collections::HashMap;

use dn_syntax::ast::File;
use dn_syntax::{Diagnostic, Span};

use crate::program::{Context, Program};
use crate::symbols::{build_table, SymbolTable};

pub(crate) use resolve::{Resolved, Scope};

/// Merges the files into one program and checks its declarations.
pub fn check(files: Vec<File>) -> Result<Program, Vec<Diagnostic>> {
    let program = Program::from_files(files);
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
    };
    for context in program.contexts.values() {
        checker.check_context(context);
    }
    for application in &program.applications {
        checker.check_application(application);
    }
    let mut errors = checker.errors;
    if errors.is_empty() {
        return Ok(program);
    }
    errors.sort_by_key(|error| (error.span.source.0, error.span.line, error.span.column));
    Err(errors)
}

pub(crate) struct Checker<'a> {
    program: &'a Program,
    tables: HashMap<String, SymbolTable>,
    errors: Vec<Diagnostic>,
}

impl<'a> Checker<'a> {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(Diagnostic::new(message, span));
    }

    fn check_context(&mut self, context: &'a Context) {
        let scope = Scope {
            context,
            aggregate: None,
        };
        self.check_names(context);
        self.check_publish(context);
        self.check_uses(context);
        for value in &context.values {
            self.check_value(&scope, value, false);
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
            self.check_function(&scope, function);
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
