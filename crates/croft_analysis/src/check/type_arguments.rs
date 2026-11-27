//! Working out a generic template's type arguments: from what it is given,
//! from an instance it is given, and written out to name an instance.

use std::collections::HashMap;

use croft_syntax::ast::{Identifier, Path, TypeExpr};

use super::{Checker, Resolved, Scope};
use crate::types::Type;

impl<'a> Checker<'a> {
    /// A type written out, so it can be an instance's type argument in the
    /// context that uses it, or `None` for a type that can't be written.
    pub(super) fn type_expr_of(&self, ty: &Type, context: &str) -> Option<TypeExpr> {
        let simple = |name: &str| TypeExpr {
            path: path_of(name),
            arguments: Vec::new(),
            number: None,
        };

        let generic = |name: &str, arguments: Vec<TypeExpr>| TypeExpr {
            path: path_of(name),
            arguments,
            number: None,
        };

        Some(match ty {
            Type::Int => simple("Int"),
            Type::Float => simple("Float"),
            Type::Bool => simple("Bool"),
            Type::String => simple("String"),
            Type::Bytes => simple("Bytes"),
            Type::Unit => simple("Unit"),
            Type::Decimal(places) => TypeExpr {
                number: Some(i64::from(*places)),
                ..simple("Decimal")
            },
            Type::List(item) => generic("List", vec![self.type_expr_of(item, context)?]),
            Type::Set(item) => generic("Set", vec![self.type_expr_of(item, context)?]),
            Type::Option(item) => generic("Option", vec![self.type_expr_of(item, context)?]),
            Type::Map(key, value) => generic(
                "Map",
                vec![
                    self.type_expr_of(key, context)?,
                    self.type_expr_of(value, context)?,
                ],
            ),
            Type::Result(value, error) => generic(
                "Result",
                vec![
                    self.type_expr_of(value, context)?,
                    self.type_expr_of(error, context)?,
                ],
            ),
            Type::Named(named) if named.context == context => simple(&named.name),
            Type::Named(named) => simple(&format!("{}.{}", named.context, named.name)),
            _ => return None,
        })
    }

    /// Works out a template's type parameters by matching a parameter's
    /// written type against the type of what is passed.
    /// `scope` is where `written` was written, the template's own.
    pub(super) fn match_type(
        &self,
        scope: &Scope,
        written: &TypeExpr,
        found: &Type,
        parameters: &[Identifier],
        bound: &mut HashMap<String, Type>,
    ) {
        let name = written.path.text();

        if parameters.iter().any(|p| p.name == name) && written.arguments.is_empty() {
            if !found.is_unknown() {
                bound.entry(name).or_insert_with(|| found.clone());
            }

            return;
        }

        let inner: Vec<Type> = match (name.as_str(), found) {
            ("List", Type::List(item))
            | ("Set", Type::Set(item))
            | ("Option", Type::Option(item)) => vec![(**item).clone()],
            ("Map", Type::Map(key, value)) | ("Result", Type::Result(key, value)) => {
                vec![(**key).clone(), (**value).clone()]
            }
            (_, Type::Named(instance)) => self.instance_arguments(scope, written, instance),
            _ => return,
        };

        for (written, found) in written.arguments.iter().zip(&inner) {
            self.match_type(scope, written, found, parameters, bound);
        }
    }

    /// The types an instance of a generic value was made with, in the order
    /// of its template's parameters, worked out from its fields. Empty when
    /// `written` doesn't name its template.
    fn instance_arguments(
        &self,
        scope: &Scope,
        written: &TypeExpr,
        instance: &Resolved,
    ) -> Vec<Type> {
        let Some(template) = self.resolve(scope, &written.path) else {
            return Vec::new();
        };

        let Some(declaration) = self.generic_value(&template) else {
            return Vec::new();
        };

        if !self.is_instance_of(&Type::Named(instance.clone()), &template) {
            return Vec::new();
        }

        let mut bound = HashMap::new();
        let template_scope = self.scope_of(&template);

        for field in &declaration.fields {
            if let Some(found) = self.field_type(instance, &field.name.name) {
                self.match_type(
                    &template_scope,
                    &field.ty,
                    &found,
                    &declaration.type_parameters,
                    &mut bound,
                );
            }
        }

        (declaration.type_parameters.iter())
            .map(|p| bound.get(&p.name).cloned().unwrap_or(Type::Unknown))
            .collect()
    }
}

/// A dotted name as a path, with no place in the source.
fn path_of(name: &str) -> Path {
    Path {
        segments: name
            .split('.')
            .map(|segment| Identifier {
                name: segment.to_string(),
                span: croft_syntax::Span::default(),
            })
            .collect(),
    }
}
