//! Naming conventions: types in PascalCase, functions and fields in snake_case and
//! constants in UPPER_SNAKE_CASE.

use croft_syntax::ast::{Field, FunctionKind, Identifier, RecordDecl};

use super::types::is_builtin_type;
use super::{Checker, Context};
use crate::naming::{is_pascal_case, is_snake_case, is_upper_snake_case};

/// A naming convention a name must follow.
#[derive(Clone, Copy)]
enum Case {
    Pascal,
    Snake,
    UpperSnake,
}

impl<'a> Checker<'a> {
    /// Reports `name` when it is not in `case`. `what` says what it names.
    fn expect_case(&mut self, name: &Identifier, case: Case, what: &str) {
        // An instance, such as `first<Int>`, is named after its template,
        // which was checked as written.
        if name.name.contains('<') {
            return;
        }

        let (matches, written) = match case {
            Case::Pascal => (is_pascal_case(&name.name), "PascalCase"),
            Case::Snake => (is_snake_case(&name.name), "snake_case"),
            Case::UpperSnake => (is_upper_snake_case(&name.name), "UPPER_SNAKE_CASE"),
        };

        if !matches {
            self.error(
                format!("{what} `{}` should be written in {written}", name.name),
                name.span,
            );
        }
    }

    /// Field names are in snake_case.
    fn expect_fields_case(&mut self, fields: &[Field]) {
        for field in fields {
            self.expect_case(&field.name, Case::Snake, "field");
        }
    }

    /// Error and event names are in PascalCase, and their fields in snake_case.
    fn expect_records_case(&mut self, records: &[RecordDecl], what: &str) {
        for record in records {
            self.expect_case(&record.name, Case::Pascal, what);
            self.expect_fields_case(&record.fields);
        }
    }

    /// `Int`, `List` and the other built-in types cannot be declared again.
    fn check_builtin_names(&mut self, context: &Context) {
        let table = &self.tables[&context.name.name];

        let clashes: Vec<_> = table
            .symbols
            .iter()
            .filter(|(name, _)| is_builtin_type(name))
            .map(|(name, symbol)| (name.clone(), symbol.span))
            .collect();

        for (name, span) in clashes {
            self.error(
                format!("`{name}` is a built-in type and cannot be declared again"),
                span,
            );
        }
    }

    /// Every name a context declares is in the case its kind uses.
    pub(super) fn check_names(&mut self, context: &Context) {
        self.check_builtin_names(context);
        self.expect_case(&context.name, Case::Pascal, "context");

        for value in &context.values {
            self.expect_case(&value.name, Case::Pascal, "value");
            self.expect_fields_case(&value.fields);
        }

        for enum_decl in &context.enums {
            self.expect_case(&enum_decl.name, Case::Pascal, "enum");
            self.expect_records_case(&enum_decl.variants, "variant");
        }

        self.expect_records_case(&context.errors, "error");

        for constant in &context.consts {
            self.expect_case(&constant.name, Case::UpperSnake, "constant");
        }

        let pascal = context
            .identities
            .iter()
            .map(|i| (&i.name, "identity"))
            .chain(context.constraints.iter().map(|c| (&c.name, "constraint")))
            .chain(context.policies.iter().map(|p| (&p.name, "policy")))
            .chain(context.ports.iter().map(|p| (&p.name, "port")))
            .chain(context.adapters.iter().map(|a| (&a.name, "adapter")))
            .chain(context.repositories.iter().map(|r| (&r.name, "repository")));

        for (name, what) in pascal.collect::<Vec<_>>() {
            self.expect_case(name, Case::Pascal, what);
        }

        for function in &context.functions {
            let (case, what) = match function.kind {
                FunctionKind::Service => (Case::Pascal, "service"),
                FunctionKind::Domain => (Case::Pascal, "domain"),
                FunctionKind::Translation => (Case::Pascal, "translation"),
                _ => (Case::Snake, "function"),
            };

            self.expect_case(&function.name, case, what);
        }

        for operation in context.ports.iter().flat_map(|port| &port.operations) {
            self.expect_case(&operation.name, Case::Snake, "operation");
        }

        for aggregate in &context.aggregates {
            self.expect_case(&aggregate.name, Case::Pascal, "aggregate");
            self.expect_fields_case(&aggregate.fields);

            for entity in &aggregate.entities {
                self.expect_case(&entity.name, Case::Pascal, "entity");
                self.expect_fields_case(&entity.fields);
            }

            self.expect_records_case(&aggregate.errors, "error");
            self.expect_records_case(&aggregate.events, "event");

            for command in &aggregate.commands {
                self.expect_case(&command.name, Case::Pascal, "command");
            }

            for factory in &aggregate.factories {
                self.expect_case(&factory.name, Case::Pascal, "factory");
            }
        }
    }
}
