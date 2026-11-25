//! Finding enum variants by name.

use croft_syntax::ast::Path;

use super::BodyChecker;
use crate::check::variant::Variant;
use crate::symbols::SymbolKind;
use crate::types::Type;

const ENUM_KINDS: [SymbolKind; 3] = [SymbolKind::Enum, SymbolKind::ErrorSet, SymbolKind::Error];

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// A variant written `Direction.North`, or just `North` where the enum is
    /// known from context or the name belongs to only one enum.
    pub(super) fn find_variant(
        &self,
        path: &Path,
        expected: Option<&Type>,
    ) -> Option<(Type, Variant<'a>)> {
        let name = path.last().name.as_str();

        let candidates: Vec<Type> = if path.segments.len() > 1 {
            let owner = Path {
                segments: path.segments[..path.segments.len() - 1].to_vec(),
            };

            let found = self.checker.resolve(&self.scope, &owner)?;

            vec![Type::Named(found)]
        } else if let Some(expected) = expected.and_then(|ty| ty.named(&ENUM_KINDS)) {
            vec![Type::Named(expected.clone())]
        } else {
            let enums = self.scope.context.enums.iter();
            let context = self.scope.context.name.name.as_str();

            enums
                .filter_map(|e| self.checker.lookup(context, &e.name.name))
                .map(Type::Named)
                .collect()
        };

        let mut matches = candidates.into_iter().filter_map(|ty| {
            let named = ty.named(&ENUM_KINDS)?.clone();

            let variant = self
                .checker
                .variants_of(&named)
                .into_iter()
                .find(|v| v.name == name)?;

            Some((ty, variant))
        });

        let first = matches.next()?;

        matches.next().is_none().then_some(first)
    }
}
