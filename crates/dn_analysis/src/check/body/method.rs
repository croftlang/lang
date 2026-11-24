//! Method calls: port operations, repositories, and the built-in methods of
//! strings, numbers, lists and options.

use dn_syntax::ast::{Argument, Identifier};

use super::BodyChecker;
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn method(
        &mut self,
        receiver: &Type,
        method: &Identifier,
        arguments: &[Argument],
        expected: Option<&Type>,
    ) -> Type {
        let name = method.name.as_str();
        let span = method.span;
        let found = match receiver {
            Type::Unknown => {
                for argument in arguments {
                    self.expr(&argument.value, None);
                }
                return Type::Unknown;
            }
            Type::Port(port) => return self.port_operation(port, method, arguments),
            Type::Repository(repository) => {
                return self.repository_operation(repository, method, arguments)
            }
            Type::String => self.string_method(name, arguments, span),
            Type::Int | Type::Float if name == "to_string" => {
                self.fixed(name, arguments, &[], Type::String, span)
            }
            Type::List(item) => self.list_method(item, name, arguments, span),
            Type::Option(item) => self.option_method(item, name, arguments, span, expected),
            _ => None,
        };
        found.unwrap_or_else(|| {
            self.error(format!("`{receiver}` has no method `{name}`"), span);
            Type::Unknown
        })
    }

    /// A method with fixed parameter types.
    pub(super) fn fixed(
        &mut self,
        name: &str,
        arguments: &[Argument],
        parameters: &[Type],
        result: Type,
        span: dn_syntax::Span,
    ) -> Option<Type> {
        let parameters: Vec<(String, Type)> = parameters
            .iter()
            .enumerate()
            .map(|(index, ty)| (format!("argument {}", index + 1), ty.clone()))
            .collect();
        self.check_arguments(arguments, &parameters, name, span, false);
        Some(result)
    }
}

/// Is this a list of entities, which are looked up by identity?
pub(super) fn entity_of(item: &Type) -> Option<&Resolved> {
    item.named(&[SymbolKind::Entity])
}
