//! Field access, and dotted paths to declarations such as `Direction.North`.

use croft_syntax::ast::{Expr, Identifier, Path};

use super::BodyChecker;
use crate::check::data::expr_path;
use crate::facts::Meaning;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Field access, or a path to a variant or error: `Direction.North`, `Room.NotFound`.
    pub(super) fn field(
        &mut self,
        value: &Expr,
        inner: &Expr,
        field: &Identifier,
        expected: Option<&Type>,
    ) -> Type {
        if let Some(path) = self.as_type_path(value) {
            if let Some(ty) = self.type_path(Some(value.id), &path, expected) {
                return ty;
            }

            return Type::Unknown;
        }

        let ty = self.expr(inner, None);

        self.note(Some(value.id), Meaning::Property);

        self.property(&ty, field)
    }

    /// A dotted expression whose first name is not a value in scope is a path to a declaration.
    pub(super) fn as_type_path(&self, value: &Expr) -> Option<Path> {
        let path = expr_path(value)?;
        let first = &path.segments[0].name;
        let is_value =
            self.local(first).is_some() || self.own_field(first, path.span(), false).is_some();
        (!is_value && first.starts_with(|c: char| c.is_ascii_uppercase())).then_some(path)
    }
}
