//! Types, fields, parameters and the constraints in `where` rules.

use std::collections::HashSet;

use dn_syntax::ast::{Field, Parameter, TypeExpr};

use super::{Checker, Scope};

const BUILTIN_TYPES: [(&str, usize); 8] = [
    ("Int", 0),
    ("Float", 0),
    ("Bool", 0),
    ("String", 0),
    ("Unit", 0),
    ("List", 1),
    ("Option", 1),
    ("Result", 2),
];

pub(super) fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.iter().any(|(builtin, _)| *builtin == name)
}

/// A single capital letter, such as the `T` in `List<T>`, in a constraint parameter.
fn is_type_parameter(name: &str) -> bool {
    name.len() == 1 && name.starts_with(|c: char| c.is_ascii_uppercase())
}

impl<'a> Checker<'a> {
    pub(super) fn check_type(&mut self, scope: &Scope, ty: &TypeExpr, generic: bool) {
        for argument in &ty.arguments {
            self.check_type(scope, argument, generic);
        }
        let name = ty.path.text();
        let span = ty.path.span();
        let found = ty.arguments.len();
        if let Some((_, expected)) = BUILTIN_TYPES.iter().find(|(builtin, _)| *builtin == name) {
            if *expected != found {
                let message = format!("`{name}` takes {expected} type arguments, found {found}");
                self.error(message, span);
            }
            return;
        }
        if generic && is_type_parameter(&name) {
            return;
        }
        if found > 0 {
            self.error(format!("`{name}` takes no type arguments"), span);
        }
        match self.resolve(scope, &ty.path) {
            None => self.error(format!("cannot find a type named `{name}`"), span),
            Some(found) if !found.kind.is_type() => {
                let actual = found.kind.describe();
                self.error(format!("expected a type, but `{name}` is {actual}"), span);
            }
            Some(_) => {}
        }
    }

    pub(super) fn check_fields(&mut self, scope: &Scope, fields: &[Field]) {
        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.name.as_str()) {
                let message = format!("field `{}` is declared more than once", field.name.name);
                self.error(message, field.name.span);
            }
            self.check_type(scope, &field.ty, false);
            self.check_rules(scope, &field.rules, Some(&field.ty));
        }
    }

    pub(super) fn check_parameters(
        &mut self,
        scope: &Scope,
        parameters: &[Parameter],
        generic: bool,
    ) {
        let mut seen = HashSet::new();
        for parameter in parameters {
            let name = &parameter.name;
            if !seen.insert(name.name.as_str()) {
                self.error(
                    format!("parameter `{}` is declared more than once", name.name),
                    name.span,
                );
            }
            if parameter.types.is_empty() {
                self.error(format!("parameter `{}` needs a type", name.name), name.span);
            }
            for ty in &parameter.types {
                self.check_type(scope, ty, generic);
            }
            self.check_rules(scope, &parameter.rules, parameter.types.first());
        }
    }
}
