//! Functions, translations, services and constraints.

use std::collections::HashMap;

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::{ConstraintUse, Expr, ExprKind};

use super::constraints::CheckPlace;

use crate::function::{Returns, Writer};
use crate::names::{c_name, c_string};
use crate::Generator;

/// How many times `settle_results` writes every function before giving up.
const RESULT_ROUNDS: usize = 8;

impl<'a> Generator<'a> {
    /// Every function, translation, service and constraint, as C.
    pub fn functions(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();

        for kind in [
            SymbolKind::Constraint,
            SymbolKind::Function,
            SymbolKind::Translation,
            SymbolKind::Service,
        ] {
            for found in self.model.all(kind) {
                out.push_str(&self.function(&found, prototypes).0);
            }
        }

        out
    }

    /// Works out what each function and translation gives back, so a short
    /// string comes back in a short string. Each round writes every function
    /// with the results the last round found, starting from their declared
    /// types, until no result changes. A result is only ever narrowed to
    /// what its body gives, so every round's results are safe to use.
    pub fn settle_results(&self) {
        let saved = self.save_state();

        for _ in 0..RESULT_ROUNDS {
            let mut found = HashMap::new();

            for kind in [SymbolKind::Function, SymbolKind::Translation] {
                for function in self.model.all(kind) {
                    if self.has_result_length(&function) {
                        continue;
                    }

                    if let (_, Some(returned)) = self.function(&function, &mut Vec::new()) {
                        found.insert(function, returned);
                    }
                }
            }

            self.restore_state(&saved);

            if found == *self.results.borrow() {
                return;
            }

            *self.results.borrow_mut() = found;
        }

        // Still changing, as recursion can make it: every result keeps its
        // declared type.
        self.results.borrow_mut().clear();
    }

    /// A function as C, and what its body was seen to give back.
    fn function(&self, found: &Resolved, prototypes: &mut Vec<String>) -> (String, Option<Type>) {
        let context = &self.program.contexts[&found.context];

        let (result, declared, preconditions, body, constraints, return_constraints) = if found.kind
            == SymbolKind::Constraint
        {
            let Some(decl) = context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name)
            else {
                return (String::new(), None);
            };

            (
                Type::Bool,
                Type::Bool,
                &[][..],
                Some(&decl.body),
                Vec::new(),
                &[][..],
            )
        } else {
            let Some(decl) = context.functions.iter().find(|f| f.name.name == found.name) else {
                return (String::new(), None);
            };

            // A generic template is compiled only as its instances.
            if !decl.type_parameters.is_empty() {
                return (String::new(), None);
            }

            let declared = self.function_result(&found.context, decl);
            let result = self.result_of(found).unwrap_or_else(|| declared.clone());

            let constraints = decl
                .parameters
                .iter()
                .flatten()
                .map(|p| p.constraints.as_slice())
                .collect();

            (
                result,
                declared,
                decl.preconditions.as_slice(),
                decl.body.as_ref(),
                constraints,
                decl.return_constraints.as_slice(),
            )
        };

        let parameters = self.parameters_of(found);
        let ranges = self.parameter_ranges(&found.context, &parameters, &constraints);
        let signature =
            self.ranged_signature(&c_name(found), &[], &parameters, &ranges, Some(&result));
        prototypes.push(format!("{signature};"));

        let mut writer = Writer::new(self, &found.context, Returns::Value(result.clone()));
        writer.label = found.name.clone();
        writer.bind_parameters(&parameters);
        writer.note_parameter_ranges(&parameters, &ranges);

        let place = CheckPlace {
            context: &found.context,
            aggregate: None,
            label: &found.name,
        };

        let checked: Vec<(String, Type, &[ConstraintUse])> = parameters
            .iter()
            .zip(constraints)
            .map(|((name, ty), constraints)| (name.clone(), ty.clone(), constraints))
            .collect();

        if let Some(call) = self.argument_check(&c_name(found), &place, &checked) {
            writer.checked_call(&call);
        }

        writer.constraint_failures(&checked);
        writer.preconditions(preconditions, &found.name);

        if let Some(body) = body {
            let value = writer.value_into("(*out)", body, &result);
            if let Some(check) =
                self.result_check(&c_name(found), &place, &result, return_constraints)
            {
                let passed = writer.pass(value.clone(), &result);
                writer.checked_call(&format!("{check}({passed});"));
            }
        }

        // A result is never kept in more than its declared type holds, so a
        // string result with no `Length` stays the target's size.
        let returned =
            (writer.returned.clone()).map(|returned| self.narrower(&returned, &declared));

        (writer.finish(&signature), returned)
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// A contract's lines: `cond or Error` fails with the error, and a plain
    /// constraint such as `from != to` is checked when checks are on.
    pub fn preconditions(&mut self, preconditions: &[Expr], owner: &str) {
        for precondition in preconditions {
            match &precondition.kind {
                ExprKind::OrElse(condition, error) => {
                    let condition = self.value(condition, &Type::Bool);
                    self.open(format!("if (!{condition})"));

                    let error = self.value(error, &Type::Errors(Vec::new()));
                    self.fail(&error);
                    self.close();
                }
                _ => {
                    let condition = self.value(precondition, &Type::Bool);
                    let message = c_string(&format!("{owner}: {} must hold", precondition.text()));

                    self.line(format!("CROFT_CHECK({condition}, {message});"));
                }
            }
        }
    }
}
