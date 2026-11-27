//! Starting values: what a field holds when it is left out of a record,
//! event or error, and what an adapter's field holds when the program starts.

use croft_syntax::ast::Field;

use super::{BodyChecker, BodyKind};
use crate::check::{Checker, Scope};

impl<'a> Checker<'a> {
    /// Checks each starting value in `fields`, written in `scope`, against
    /// its field's type, and its constraints when the value is known. A
    /// starting value is worked out on its own, so it can name constants and
    /// variants but not the other fields. `owner` names the declaration in
    /// messages, such as `Room.Player`.
    pub(super) fn check_starting_values(
        &mut self,
        scope: Scope<'a>,
        owner: &str,
        fields: &[Field],
    ) {
        for field in fields {
            let Some(value) = &field.value else { continue };
            let ty = self.lower_type(&scope, &field.ty);

            let mut body = BodyChecker::new(self, scope, BodyKind::Constant, ty.clone());
            body.expect(value, &ty);

            let given = format!("`{owner}` starts with");
            body.check_known_value(value, &field.constraints, &scope, &given, &field.name.name);
        }
    }
}
