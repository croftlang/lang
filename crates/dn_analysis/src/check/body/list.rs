//! The built-in methods of lists.

use dn_syntax::ast::Argument;
use dn_syntax::Span;

use super::method::entity_of;
use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A list of entities is also looked up, replaced and removed by identity.
    pub(super) fn list_method(
        &mut self,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        span: Span,
    ) -> Option<Type> {
        let list = Type::list(item.clone());
        let key = match entity_of(item) {
            Some(entity) => self.checker.identity_of(entity),
            None => Type::Int,
        };
        match name {
            "contains" if entity_of(item).is_some() => {
                self.fixed(name, arguments, &[key], Type::Bool, span)
            }
            "contains" => self.fixed(
                name,
                arguments,
                std::slice::from_ref(item),
                Type::Bool,
                span,
            ),
            "get" if entity_of(item).is_some() => {
                self.fixed(name, arguments, &[key], item.clone(), span)
            }
            "get" => self.fixed(
                name,
                arguments,
                &[Type::Int],
                Type::option(item.clone()),
                span,
            ),
            "remove" if entity_of(item).is_some() => {
                self.fixed(name, arguments, &[key], list, span)
            }
            "replace" if entity_of(item).is_some() => {
                self.fixed(name, arguments, std::slice::from_ref(item), list, span)
            }
            "add" => self.fixed(name, arguments, std::slice::from_ref(item), list, span),
            "drop" | "take" => self.fixed(name, arguments, &[Type::Int], list, span),
            "filter" => {
                self.lambda_returning(name, arguments, item, &Type::Bool, span);
                Some(list)
            }
            "all" | "any" => {
                self.lambda_returning(name, arguments, item, &Type::Bool, span);
                Some(Type::Bool)
            }
            "count" => {
                self.lambda_returning(name, arguments, item, &Type::Bool, span);
                Some(Type::Int)
            }
            "find" => {
                self.lambda_returning(name, arguments, item, &Type::Bool, span);
                Some(Type::option(item.clone()))
            }
            "sum" => {
                self.lambda_returning(name, arguments, item, &Type::Int, span);
                Some(Type::Int)
            }
            "min_by" | "max_by" => {
                self.lambda_returning(name, arguments, item, &Type::Int, span);
                Some(Type::option(item.clone()))
            }
            "map" => Some(Type::list(
                self.lambda_argument(name, arguments, item, span),
            )),
            _ => None,
        }
    }

    pub(super) fn lambda_returning(
        &mut self,
        name: &str,
        arguments: &[Argument],
        item: &Type,
        result: &Type,
        span: Span,
    ) {
        let found = self.lambda_argument(name, arguments, item, span);
        if !self.checker.accepts(result, &found) {
            self.error(
                format!("the lambda passed to `{name}` must give back `{result}`, found `{found}`"),
                span,
            );
        }
    }

    /// `[a, b]`: every item has the same type. `[]` takes its item type from
    /// where it is used.
    pub(super) fn list_literal(
        &mut self,
        items: &[dn_syntax::ast::Expr],
        expected: Option<&Type>,
    ) -> Type {
        let expected_item = match expected {
            Some(Type::List(item)) if !item.is_unknown() => Some((**item).clone()),
            _ => None,
        };
        let Some((first, rest)) = items.split_first() else {
            return Type::list(expected_item.unwrap_or(Type::Unknown));
        };
        let item = match expected_item {
            Some(item) => {
                self.expect(first, &item);
                item
            }
            None => self.expr(first, None),
        };
        for other in rest {
            self.expect(other, &item);
        }
        Type::list(item)
    }
}
