//! The properties of built-in and declared types: `text.length`, `room.player`, `event.source`.

use b_syntax::ast::Identifier;

use super::BodyChecker;
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `text.length`, `enemies.is_empty`, `room.player`, `event.source`.
    pub(super) fn property(&mut self, ty: &Type, field: &Identifier) -> Type {
        let name = field.name.as_str();
        let found = match (ty, name) {
            (Type::Unknown, _) => Some(Type::Unknown),
            (Type::String | Type::Bytes | Type::List(_), "length") => Some(Type::Int),
            (Type::String | Type::Bytes | Type::List(_), "is_empty") => Some(Type::Bool),
            (Type::List(item), "first" | "last") => Some(Type::option((**item).clone())),
            (Type::Map(..), "length") => Some(Type::Int),
            (Type::Map(..), "is_empty") => Some(Type::Bool),
            (Type::Map(key, _), "keys") => Some(Type::list((**key).clone())),
            (Type::Map(_, value), "values") => Some(Type::list((**value).clone())),
            (Type::Option(_), "is_some" | "is_none") => Some(Type::Bool),
            (Type::Named(named), "source") if named.kind == SymbolKind::Event => {
                let aggregate = named.name.split('.').next().unwrap_or_default();
                let aggregate = self.checker.lookup(&named.context, aggregate);
                aggregate.map(|aggregate| self.checker.identity_of(&aggregate))
            }
            (Type::Named(named), _) => self.checker.field_type(named, name),
            _ => None,
        };
        found.unwrap_or_else(|| {
            self.error(format!("`{ty}` has no field `{name}`"), field.span);
            Type::Unknown
        })
    }
}
