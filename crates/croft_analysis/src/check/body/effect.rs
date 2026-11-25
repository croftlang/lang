//! Raising events and calling native code.

use croft_syntax::ast::{Argument, FieldValue, Identifier, Path};
use croft_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `emit EnemyDamaged { ... }`, in a command, for one of its aggregate's events.
    pub(super) fn emit(&mut self, event: &Path, fields: &[FieldValue], span: Span) -> Type {
        let BodyKind::Command(aggregate) = self.kind else {
            self.error("only a command can raise an event", span);
            return Type::Unit;
        };
        let name = format!("{}.{}", aggregate.name.name, event.text());
        let Some(found) = self
            .checker
            .lookup(&self.scope.context.name.name, &name)
            .filter(|f| f.kind == SymbolKind::Event)
        else {
            self.error(
                format!("`{}` has no event `{}`", aggregate.name.name, event.text()),
                event.span(),
            );
            return Type::Unit;
        };
        let declared = self.checker.fields_of(&found).unwrap_or(&[]);
        for field in declared {
            if !fields
                .iter()
                .any(|given| given.name.name == field.name.name)
            {
                self.error(
                    format!("`{}` needs a value for `{}`", event.text(), field.name.name),
                    span,
                );
            }
        }
        self.field_values(&Type::Named(found), fields);
        Type::Unit
    }

    /// `native zx_cls()`: C code, only from an adapter. Its type comes from the port.
    pub(super) fn native(
        &mut self,
        function: &Identifier,
        arguments: &[Argument],
        span: Span,
        expected: Option<&Type>,
    ) -> Type {
        if !matches!(self.kind, BodyKind::Adapter(_)) {
            self.error(
                format!(
                    "only an adapter can call native code such as `{}`",
                    function.name
                ),
                span,
            );
        }
        for argument in arguments {
            self.expr(&argument.value, None);
        }
        expected.cloned().unwrap_or(Type::Unknown)
    }
}
