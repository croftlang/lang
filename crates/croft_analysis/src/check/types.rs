//! Types, fields, parameters and the constraints in `where` clauses.

use std::collections::HashSet;

use croft_syntax::ast::{Field, Parameter, TypeExpr};

use super::constraints::ConstraintPlace;
use super::{Checker, Scope};
use crate::facts::BuiltinName;
use crate::types::MAX_PLACES;

/// The built-in types, each with how many type arguments it takes.
const BUILTIN_TYPES: [(&str, usize); 12] = [
    ("Int", 0),
    ("Float", 0),
    ("Decimal", 0),
    ("Bool", 0),
    ("String", 0),
    ("Bytes", 0),
    ("Unit", 0),
    ("List", 1),
    ("Map", 2),
    ("Set", 1),
    ("Option", 1),
    ("Result", 2),
];

/// Whether `name` is a built-in type.
pub(super) fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.iter().any(|(builtin, _)| *builtin == name)
}

/// A single capital letter, such as the `T` in `List<T>`, in a constraint parameter.
fn is_type_parameter(name: &str) -> bool {
    name.len() == 1 && name.starts_with(|c: char| c.is_ascii_uppercase())
}

impl<'a> Checker<'a> {
    /// A written type names a type, with as many type arguments as it takes.
    /// With `generic`, a type parameter such as `T` is a type too.
    pub(super) fn check_type(&mut self, scope: &Scope, ty: &TypeExpr, generic: bool) {
        for argument in &ty.arguments {
            self.check_type(scope, argument, generic);
        }

        let name = ty.path.text();
        let span = ty.path.span();
        let found = ty.arguments.len();

        if name == "Map" || name == "Set" {
            self.check_standard(scope.context, &name, span);
        }

        if name == "Decimal" {
            self.check_standard(scope.context, &name, span);
            self.check_places(ty);
        } else if ty.number.is_some() {
            self.error(format!("`{name}` takes no number"), span);
        }

        if let Some((_, expected)) = BUILTIN_TYPES.iter().find(|(builtin, _)| *builtin == name) {
            self.facts.builtins.push(BuiltinName {
                span,
                name: name.clone(),
                from: None,
            });

            if *expected != found {
                let message = format!("`{name}` takes {expected} type arguments, found {found}");
                self.error(message, span);
            }

            return;
        }

        if generic && is_type_parameter(&name) {
            return;
        }

        let resolved = self.resolve(scope, &ty.path);
        let template = resolved.as_ref().and_then(|r| self.generic_value(r));

        match template {
            Some(template) if found != template.type_parameters.len() => {
                let parameters: Vec<&str> = (template.type_parameters.iter())
                    .map(|p| p.name.as_str())
                    .collect();

                let message = format!(
                    "`{name}` takes {} type arguments, for `{name}<{}>`, found {found}",
                    parameters.len(),
                    parameters.join(", ")
                );

                self.error(message, span);
            }
            None if found > 0 => self.error(format!("`{name}` takes no type arguments"), span),
            _ => {}
        }

        match resolved {
            None => self.error(crate::standard::not_found_message("a type", &name), span),
            Some(found) if !found.kind.is_type() => {
                let actual = found.kind.describe();
                self.error(format!("expected a type, but `{name}` is {actual}"), span);
            }
            Some(_) => {}
        }
    }

    /// `Decimal<2>`: the places must be given, from 1 to `MAX_PLACES`.
    fn check_places(&mut self, ty: &TypeExpr) {
        let span = ty.path.span();

        match ty.number {
            None => self.error(
                "`Decimal` needs its decimal places, as in `Decimal<2>`",
                span,
            ),
            Some(places) if !(1..=MAX_PLACES).contains(&places) => self.error(
                format!("`Decimal` has 1 to {MAX_PLACES} decimal places, found {places}"),
                span,
            ),
            Some(_) => {}
        }
    }

    /// Each field is declared once, and its type and constraints are checked.
    pub(super) fn check_fields(&mut self, scope: &Scope, fields: &[Field]) {
        let mut seen = HashSet::new();

        for field in fields {
            if !seen.insert(field.name.name.as_str()) {
                let message = format!("field `{}` is declared more than once", field.name.name);
                self.error(message, field.name.span);
            }

            self.check_type(scope, &field.ty, false);
            let place = ConstraintPlace::Field;
            self.check_constraints(scope, &field.constraints, Some(&field.ty), place);

            if let Some(value) = &field.value {
                self.error("only an adapter's fields have starting values", value.span);
            }
        }
    }

    /// Each parameter is declared once, with a type, and constraints that fit `place`.
    pub(super) fn check_parameters(
        &mut self,
        scope: &Scope,
        parameters: &[Parameter],
        generic: bool,
        place: ConstraintPlace,
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

            self.check_constraints(
                scope,
                &parameter.constraints,
                parameter.types.first(),
                place,
            );
        }
    }
}
