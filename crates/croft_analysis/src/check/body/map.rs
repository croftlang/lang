//! `Map<K, V>`, imported from `Std.Collections`: literals and methods. A map
//! is a value like any other, so `set` and `remove` give back a changed copy.

use croft_syntax::ast::{Argument, Expr};
use croft_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `["sword": 3, "key": 1]`, or `[:]` with its types from where it goes.
    pub(super) fn map_literal(
        &mut self,
        entries: &[(Expr, Expr)],
        expected: Option<&Type>,
        span: Span,
    ) -> Type {
        let (mut key, mut value) = match expected {
            Some(Type::Map(key, value)) => (Some((**key).clone()), Some((**value).clone())),
            _ => (None, None),
        };
        for (entry_key, entry_value) in entries {
            key = Some(self.typed_entry(entry_key, key.take()));
            value = Some(self.typed_entry(entry_value, value.take()));
        }
        if entries.is_empty() && key.is_none() {
            self.error("an empty map needs its type from where it goes, such as `let stock: Map<String, Int> = [:]`", span);
        }
        Type::map(key.unwrap_or(Type::Unknown), value.unwrap_or(Type::Unknown))
    }

    fn typed_entry(&mut self, value: &Expr, known: Option<Type>) -> Type {
        match known.filter(|ty| !ty.is_unknown()) {
            Some(ty) => {
                self.expect(value, &ty);
                ty
            }
            None => self.expr(value, None),
        }
    }

    /// `get` gives the value for a key, if there is one; `set` a copy with the
    /// key's value set; `remove` a copy without the key; `contains` whether
    /// the key is there.
    pub(super) fn map_method(
        &mut self,
        key: &Type,
        value: &Type,
        name: &str,
        arguments: &[Argument],
        span: Span,
    ) -> Option<Type> {
        let map = Type::map(key.clone(), value.clone());
        let key = key.clone();
        match name {
            "get" => self.fixed(name, arguments, &[key], Type::option(value.clone()), span),
            "set" => self.fixed(name, arguments, &[key, value.clone()], map, span),
            "remove" => self.fixed(name, arguments, &[key], map, span),
            "contains" => self.fixed(name, arguments, &[key], Type::Bool, span),
            _ => None,
        }
    }
}
