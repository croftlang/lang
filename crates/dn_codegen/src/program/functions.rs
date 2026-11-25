//! Functions, translations, services and constraints.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::{Expr, ExprKind, Rule};

use super::rules::CheckPlace;

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
        let (result, preconditions, body, rules, return_rules) = if found.kind
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
            let result = self.function_result(&found.context, decl);
            let rules = decl
                .parameters
                .iter()
                .flatten()
                .map(|p| p.rules.as_slice())
                .collect();
            (
                result,
                decl.preconditions.as_slice(),
                decl.body.as_ref(),
                rules,
                decl.return_rules.as_slice(),
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
        let checked: Vec<(String, Type, &[Rule])> = parameters
            .iter()
            .zip(rules)
            .map(|((name, ty), rules)| (name.clone(), ty.clone(), rules))
            .collect();
        if let Some(call) = self.argument_check(&c_name(found), &place, &checked) {
            writer.debug_call(&call);
        }
        writer.preconditions(preconditions, &found.name);
        if let Some(body) = body {
            let value = writer.value(body, &result);
            if let Some(check) = self.result_check(&c_name(found), &place, &result, return_rules) {
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
                        "DN_CHECK({condition}, \"a precondition of {owner} failed\");"
                    ));
                }
            }
        }
    }
}
