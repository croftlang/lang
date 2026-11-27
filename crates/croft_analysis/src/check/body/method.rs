//! Method calls: port operations, repositories, and the built-in methods of
//! strings, numbers, lists, maps, sets, options and results.

use croft_syntax::ast::{Argument, Identifier};

use super::BodyChecker;
use crate::facts::MemberKind;
use crate::standard::{ENCODING_CONTEXT, TEXT_CONTEXT};
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of a method call, chosen by the receiver's type.
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
            Type::Bytes => {
                let context = self.scope.context;

                // A name two standard contexts provide is checked against
                // the one whose method `Bytes` has.
                if let Some(standard) = bytes_standard(name) {
                    self.checker
                        .check_standard_from(context, standard, name, span);
                }

                self.bytes_method(name, arguments, span)
            }
            Type::String => {
                let context = self.scope.context;
                self.checker.check_standard(context, name, span);
                self.string_method(name, arguments, span, expected)
            }
            Type::Int if name == "to_hex" => {
                let context = self.scope.context;

                // `to_hex` on `Bytes` is `Std.Encoding`'s, so it doesn't count.
                self.checker
                    .check_standard_from(context, TEXT_CONTEXT, name, span);

                self.fixed(name, arguments, &[], Type::String, span)
            }
            Type::Int | Type::Float | Type::Decimal(_) if name == "to_string" => {
                self.fixed(name, arguments, &[], Type::String, span)
            }
            Type::Decimal(_) if name == "round" => {
                self.fixed(name, arguments, &[], Type::Int, span)
            }
            Type::List(item) => self.list_method(item, name, arguments, span, expected),
            Type::Map(key, value) => self.map_method(key, value, name, arguments, span),
            Type::Set(item) => self.set_method(item, name, arguments, span),
            Type::Option(item) => self.option_method(item, name, arguments, span, expected),
            Type::Result(value, error) => {
                self.result_method((value, error), name, arguments, span, expected)
            }
            _ => None,
        };

        let Some(found) = found else {
            self.error(format!("`{receiver}` has no method `{name}`"), span);
            return Type::Unknown;
        };

        self.note_member(method, receiver, &found, MemberKind::Operation, None);

        found
    }

    /// A method with fixed parameter types.
    pub(super) fn fixed(
        &mut self,
        name: &str,
        arguments: &[Argument],
        parameters: &[Type],
        result: Type,
        span: croft_syntax::Span,
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

/// The standard context a method of `Bytes` called `name` is imported from,
/// if it is imported at all.
fn bytes_standard(name: &str) -> Option<&'static str> {
    match name {
        "to_text" => Some(TEXT_CONTEXT),
        "to_hex" | "to_base64" | "crc16" | "crc32" => Some(ENCODING_CONTEXT),
        _ => None,
    }
}

/// The entity a list holds, when its items are entities, which are looked up by identity.
pub(super) fn entity_of(item: &Type) -> Option<&Resolved> {
    item.named(&[SymbolKind::Entity])
}
