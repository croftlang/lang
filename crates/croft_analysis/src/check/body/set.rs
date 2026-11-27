//! `Set<T>`, imported from `Std.Collections`: literals and methods. A set
//! holds each item once and, like a map, is a value like any other, so `add`
//! and `remove` give back a changed copy.

use croft_syntax::ast::{Argument, Expr};
use croft_syntax::Span;

use super::BodyChecker;
use crate::check::known::Known;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `[1, 2, 3]` where a set is expected, or `[]` for an empty one. An item
    /// known to be listed twice is a mistake, since a set holds it once.
    pub(super) fn set_literal(&mut self, items: &[Expr], expected: Option<&Type>) -> Type {
        let item = match expected {
            Some(Type::Set(item)) => (**item).clone(),
            _ => Type::Unknown,
        };

        let mut known: Vec<Known> = Vec::new();

        for value in items {
            self.expect(value, &item);

            let Some(value_known) = self.known_local_value(value) else {
                continue;
            };

            if known.contains(&value_known) {
                let message = format!(
                    "`{}` is in this set more than once, but a set holds each item once",
                    value_known.describe()
                );

                self.error(message, value.span);
                continue;
            }

            known.push(value_known);
        }

        Type::set(item)
    }

    /// `add` gives a copy with the item in it, `remove` a copy without it, and
    /// `contains` whether it is there.
    pub(super) fn set_method(
        &mut self,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        span: Span,
    ) -> Option<Type> {
        let set = Type::set(item.clone());
        let item = std::slice::from_ref(item);

        match name {
            "add" | "remove" => self.fixed(name, arguments, item, set, span),
            "contains" => self.fixed(name, arguments, item, Type::Bool, span),
            _ => None,
        }
    }
}
