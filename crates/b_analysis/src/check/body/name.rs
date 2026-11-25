//! Names, fields, records and `with`.

use b_syntax::ast::{ExprId, Identifier, Path};
use b_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::facts::{Meaning, MemberKind};
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    pub(super) fn name(
        &mut self,
        id: Option<ExprId>,
        name: &str,
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        if let Some(local) = self.local(name) {
            let (ty, defined) = (local.ty.clone(), local.defined);
            self.note(id, Meaning::Local);
            self.note_local(name, &ty, span, defined);
            return ty;
        }
        if let Some(ty) = self.own_field(name, span, false) {
            self.note(id, Meaning::OwnField);
            self.note_own_field(name, span, &ty);
            return ty;
        }
        match name {
            "Unit" => {
                self.note(id, Meaning::Unit);
                return Type::Unit;
            }
            "None" => {
                self.note(id, Meaning::NoneValue);
                return Type::option(Type::Unknown);
            }
            _ => {}
        }
        let path = Path {
            segments: vec![Identifier {
                name: name.to_string(),
                span,
            }],
        };
        if let Some(ty) = self.type_path(id, &path, expected) {
            return ty;
        }
        self.error(format!("cannot find `{name}` here"), span);
        Type::Unknown
    }

    /// Records the use of a field of the command's aggregate or the adapter.
    pub(super) fn note_own_field(&mut self, name: &str, span: Span, ty: &Type) {
        let (owner, fields) = match self.kind {
            BodyKind::Command(aggregate) => (&aggregate.name.name, &aggregate.fields),
            BodyKind::Adapter(adapter) => (&adapter.name.name, &adapter.fields),
            _ => return,
        };
        let defined = fields
            .iter()
            .find(|f| f.name.name == name)
            .map(|f| f.name.span);
        let context = &self.scope.context.name.name;
        let receiver = self
            .checker
            .lookup(context, owner)
            .map_or(Type::Unknown, Type::Named);
        let member = Identifier {
            name: name.to_string(),
            span,
        };
        self.note_member(&member, &receiver, ty, MemberKind::Field, defined);
    }

    /// A field of the command's aggregate or of the adapter. `for_set` leaves
    /// out the identity, which never changes.
    pub(super) fn own_field(&self, name: &str, _span: Span, for_set: bool) -> Option<Type> {
        let fields = match self.kind {
            BodyKind::Command(aggregate) => &aggregate.fields,
            BodyKind::Adapter(adapter) => &adapter.fields,
            _ => return None,
        };
        let field = fields
            .iter()
            .find(|field| field.name.name == name && !(for_set && field.is_identity))?;
        Some(self.checker.lower_type(&self.scope, &field.ty))
    }
}
