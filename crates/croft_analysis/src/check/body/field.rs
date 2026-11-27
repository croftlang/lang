//! Field access, and dotted paths to declarations such as `Direction.North`.

use croft_syntax::ast::{Expr, Identifier, Path};

use super::BodyChecker;
use crate::check::data::expr_path;
use crate::facts::Meaning;
use crate::symbols::SymbolKind;
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
    /// A constant is a value, so `BIG.to_string()` and `Limits.BIG.to_string()` are methods.
    pub(super) fn as_type_path(&self, value: &Expr) -> Option<Path> {
        let path = expr_path(value)?;
        let first = &path.segments[0].name;
        let is_value = self.local(first).is_some()
            || self.own_field(first, path.span(), false).is_some()
            || self.starts_with_constant(&path);
        (!is_value && first.starts_with(|c: char| c.is_ascii_uppercase())).then_some(path)
    }

    /// Whether the path, without its last name, starts with a constant.
    fn starts_with_constant(&self, path: &Path) -> bool {
        (1..path.segments.len()).any(|length| {
            let prefix = Path {
                segments: path.segments[..length].to_vec(),
            };

            (self.checker.find_path(&self.scope, &prefix))
                .is_some_and(|found| found.kind == SymbolKind::Const)
        })
    }
}
