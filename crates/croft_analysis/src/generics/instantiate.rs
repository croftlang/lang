//! Making an instance: a copy of a template with its type parameters
//! replaced by the requested types, named after them.

use std::collections::HashMap;

use croft_syntax::ast::{ContextDecl, ContextItem, File, Identifier, TypeExpr};
use croft_syntax::SourceId;

use super::renumber::renumber;
use super::Request;

/// A file holding the instance a request asks for, in the context that uses
/// it, or `None` when the template can't be found in `files`.
pub fn instance_file(request: &Request, files: &[File], source: SourceId) -> Option<File> {
    let item = files
        .iter()
        .flat_map(|file| &file.contexts)
        .filter(|context| context.name.name == request.template_context)
        .flat_map(|context| &context.items)
        .find_map(|item| match item {
            ContextItem::Value(value)
                if value.name.name == request.template && !value.type_parameters.is_empty() =>
            {
                Some(ContextItem::Value(value.clone()))
            }
            ContextItem::Function(function)
                if function.name.name == request.template
                    && !function.type_parameters.is_empty() =>
            {
                Some(ContextItem::Function(function.clone()))
            }
            _ => None,
        })?;

    let parameters = match &item {
        ContextItem::Value(value) => &value.type_parameters,
        ContextItem::Function(function) => &function.type_parameters,
        _ => return None,
    };

    let bound: HashMap<String, TypeExpr> = (parameters.iter())
        .map(|p| p.name.clone())
        .zip(request.arguments.iter().cloned())
        .collect();

    let substitute = |ty: &mut TypeExpr| write_in(ty, &bound);
    let name = request.name();

    let item = match item {
        ContextItem::Value(mut value) => {
            value.type_parameters.clear();
            value.name.name = name;

            let mut next = 0;

            for field in &mut value.fields {
                substitute(&mut field.ty);

                for constraint in &mut field.constraints {
                    for argument in constraint.arguments.iter_mut().chain(&mut constraint.error) {
                        renumber(argument, source, &mut next, &substitute);
                    }
                }
            }

            ContextItem::Value(value)
        }
        ContextItem::Function(mut function) => {
            function.type_parameters.clear();
            function.name.name = name;

            let mut next = 0;

            for parameter in function.parameters.iter_mut().flatten() {
                parameter.types.iter_mut().for_each(substitute);

                for constraint in &mut parameter.constraints {
                    for argument in constraint.arguments.iter_mut().chain(&mut constraint.error) {
                        renumber(argument, source, &mut next, &substitute);
                    }
                }
            }

            if let Some(ty) = &mut function.return_type {
                substitute(ty);
            }

            for constraint in &mut function.return_constraints {
                for argument in constraint.arguments.iter_mut().chain(&mut constraint.error) {
                    renumber(argument, source, &mut next, &substitute);
                }
            }

            if let Some(body) = &mut function.body {
                renumber(body, source, &mut next, &substitute);
            }

            for precondition in &mut function.preconditions {
                renumber(precondition, source, &mut next, &substitute);
            }

            ContextItem::Function(function)
        }
        _ => return None,
    };

    Some(File {
        contexts: vec![ContextDecl {
            doc: Vec::new(),
            name: Identifier {
                name: request.context.clone(),
                span: match &item {
                    ContextItem::Value(value) => value.name.span,
                    ContextItem::Function(function) => function.name.span,
                    _ => return None,
                },
            },
            items: vec![item],
        }],
        domains: Vec::new(),
    })
}

/// Replaces each type parameter in `ty`, however deep, with its type.
fn write_in(ty: &mut TypeExpr, bound: &HashMap<String, TypeExpr>) {
    if ty.path.segments.len() == 1 && ty.arguments.is_empty() {
        if let Some(argument) = bound.get(&ty.path.segments[0].name) {
            *ty = argument.clone();
            return;
        }
    }

    for argument in &mut ty.arguments {
        write_in(argument, bound);
    }
}
