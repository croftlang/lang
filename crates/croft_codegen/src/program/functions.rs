//! Functions, translations, services and constraints.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::{ConstraintUse, Expr, ExprKind};

use super::constraints::CheckPlace;

use crate::function::{Returns, Writer};
use crate::names::c_name;
use crate::Generator;

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
                out.push_str(&self.function(&found, prototypes));
            }
        }
        out
    }

    fn function(&self, found: &Resolved, prototypes: &mut Vec<String>) -> String {
        let context = &self.program.contexts[&found.context];
        let (result, preconditions, body, constraints, return_constraints) = if found.kind
            == SymbolKind::Constraint
        {
            let Some(decl) = context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name)
            else {
                return String::new();
            };
            (Type::Bool, &[][..], Some(&decl.body), Vec::new(), &[][..])
        } else {
            let Some(decl) = context.functions.iter().find(|f| f.name.name == found.name) else {
                return String::new();
            };
            // A generic template is compiled only as its instances.
            if !decl.type_parameters.is_empty() {
                return String::new();
            }
            let result = self.function_result(&found.context, decl);
            let constraints = decl
                .parameters
                .iter()
                .flatten()
                .map(|p| p.constraints.as_slice())
                .collect();
            (
                result,
                decl.preconditions.as_slice(),
                decl.body.as_ref(),
                constraints,
                decl.return_constraints.as_slice(),
            )
        };
        let parameters = self.parameters_of(found);
        let signature = self.signature(&c_name(found), &[], &parameters, Some(&result));
        prototypes.push(format!("{signature};"));
        let mut writer = Writer::new(self, &found.context, Returns::Value(result.clone()));
        writer.bind_parameters(&parameters);
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
            writer.debug_call(&call);
        }
        writer.preconditions(preconditions, &found.name);
        if let Some(body) = body {
            let value = writer.value(body, &result);
            if let Some(check) =
                self.result_check(&c_name(found), &place, &result, return_constraints)
            {
                let passed = writer.pass(value.clone(), &result);
                writer.debug_call(&format!("{check}({passed});"));
            }
            writer.line(format!("*out = {value};"));
        }
        writer.finish(&signature)
    }
}

impl<'g, 'a> Writer<'g, 'a> {
    /// `cond or Error` fails with the error; a plain condition is checked in debug builds.
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
                    self.line(format!(
                        "CROFT_CHECK({condition}, \"a precondition of {owner} failed\");"
                    ));
                }
            }
        }
    }
}
