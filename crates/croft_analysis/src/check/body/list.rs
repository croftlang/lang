//! The built-in methods of lists.

use croft_syntax::ast::{Argument, ExprKind};
use croft_syntax::Span;

use super::method::entity_of;
use super::BodyChecker;
use crate::check::known::Known;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A list of entities is also looked up, replaced and removed by identity.
    pub(super) fn list_method(
        &mut self,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
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
            "index_of" if entity_of(item).is_none() => self.fixed(
                name,
                arguments,
                std::slice::from_ref(item),
                Type::option(Type::Int),
                span,
            ),
            "join" if *item == Type::String => {
                let context = self.scope.context;
                self.checker.check_standard(context, name, span);
                self.fixed(name, arguments, &[Type::String], Type::String, span)
            }
            "add" => self.fixed(name, arguments, std::slice::from_ref(item), list, span),
            "insert" => {
                self.check_insert_place(arguments);
                self.fixed(name, arguments, &[Type::Int, item.clone()], list, span)
            }
            "drop" | "take" => self.fixed(name, arguments, &[Type::Int], list, span),
            "reverse" | "unique" => self.fixed(name, arguments, &[], list, span),
            // Numbers by size, and strings byte by byte.
            "sort" if item.is_number() || *item == Type::String => {
                self.fixed(name, arguments, &[], list, span)
            }
            "sort_by" => {
                self.lambda_returning(name, arguments, item, &Type::Int, span);
                Some(list)
            }
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
            "fold" => Some(self.fold(arguments, item, expected, span)),
            _ => None,
        }
    }

    /// `items.fold(start, (total, item) -> ...)`: the lambda is given the
    /// total so far, `start` at first, and each item in turn, and gives back
    /// the next total. The last total is the result.
    fn fold(
        &mut self,
        arguments: &[Argument],
        item: &Type,
        expected: Option<&Type>,
        span: Span,
    ) -> Type {
        let example = "(total, item) -> total + item";

        let [start, lambda] = arguments else {
            self.error(
                format!("`fold` takes 2 arguments, a start and a lambda such as `{example}`"),
                span,
            );

            return Type::Unknown;
        };

        let total = self.expr(&start.value, expected);

        let empty = matches!(&start.value.kind, ExprKind::List(items) if items.is_empty());

        if empty && matches!(&total, Type::List(item) if item.is_unknown()) {
            self.error(
                "an empty list to start `fold` with needs its type from where the result goes, \
                 such as `let names: List<String> = items.fold([], ...)`",
                start.value.span,
            );

            return Type::Unknown;
        }

        let found = self.lambda_with(
            "fold",
            &lambda.value,
            &[total.clone(), item.clone()],
            example,
        );

        if !self.checker.accepts(&total, &found) {
            self.error(
                format!("the lambda passed to `fold` must give back `{total}`, found `{found}`"),
                span,
            );
        }

        total
    }

    /// Reports `insert` at a place known to be below zero, which no list has.
    fn check_insert_place(&mut self, arguments: &[Argument]) {
        let Some(place) = arguments.first() else {
            return;
        };

        let Some(Known::Number { units, places: 0 }) = self.known_local_value(&place.value) else {
            return;
        };

        if units < 0 {
            self.error(
                format!("`insert` needs a place from 0 to the list's length, found {units}"),
                place.value.span,
            );
        }
    }

    /// The single lambda argument of a list method, which must give back `result`.
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
        items: &[croft_syntax::ast::Expr],
        expected: Option<&Type>,
    ) -> Type {
        if expected == Some(&Type::Bytes) {
            return self.bytes_literal(items);
        }

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
