//! Identities: the types that tell entities and aggregates apart.

use super::{Checker, Resolved};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// Whether a type is an identity built on `Int`, such as `RoomId`.
    pub(super) fn is_int_identity(&self, ty: &Type) -> bool {
        let Some(named) = ty.named(&[SymbolKind::Identity]) else {
            return false;
        };
        let identities = &self.program.contexts[&named.context].identities;
        identities
            .iter()
            .any(|i| i.name.name == named.name && i.ty.path.text() == "Int")
    }

    /// The type of an aggregate's identity field, such as `RoomId` for `Room`.
    pub(super) fn identity_of(&self, aggregate: &Resolved) -> Type {
        let fields = self.fields_of(aggregate).unwrap_or(&[]);
        let Some(identity) = fields.iter().find(|field| field.is_identity) else {
            return Type::Unknown;
        };
        self.lower_type(&self.scope_of(aggregate), &identity.ty)
    }
}
