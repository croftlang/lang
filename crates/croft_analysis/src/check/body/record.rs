//! Records, `with`, and the field values they are given.

use croft_syntax::ast::{Expr, FieldValue, Path};

use super::{BodyChecker, BodyKind};
use crate::symbols::SymbolKind;
use std::collections::HashMap;

use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Checks each field value against the field it names in `ty`.
    pub(super) fn field_values(&mut self, ty: &Type, fields: &[FieldValue]) {
        let mut seen = Vec::new();

        for given in fields {
            let name = &given.name;

            if seen.contains(&name.name) {
                self.error(
                    format!("`{}` is given more than once", name.name),
                    name.span,
                );
            }

            seen.push(name.name.clone());

            let field_type = match ty {
                Type::Named(named) => self.checker.field_type(named, &name.name),
                Type::Unknown => Some(Type::Unknown),
                _ => None,
            };

            let Some(field_type) = field_type else {
                self.error(format!("`{ty}` has no field `{}`", name.name), name.span);
                continue;
            };

            let defined = self.field_span(ty, &name.name);
            let kind = crate::facts::MemberKind::Field;
            self.note_member(name, ty, &field_type, kind, defined);

            if let Type::Named(named) = ty {
                self.check_known_field(named, given);
            }

            match &given.value {
                Some(value) => {
                    self.expect(value, &field_type);
                }
                None => {
                    let found = self.name(None, &name.name, name.span, Some(&field_type));

                    if !self.checker.accepts(&field_type, &found) {
                        let message = format!(
                            "expected `{field_type}` for `{}`, found `{found}`",
                            name.name
                        );

                        self.error(message, name.span);
                    }
                }
            }
        }
    }

    /// `Position { x: 10, y: 20 }`: every field without a starting value, and
    /// each field at most once. A generic value,
    /// `Pair { first: 1, second: "a" }`, is the instance its fields, or the
    /// type expected, call for.
    pub(super) fn record(
        &mut self,
        path: &Path,
        fields: &[FieldValue],
        expected: Option<&Type>,
    ) -> Type {
        let kinds = [
            SymbolKind::Value,
            SymbolKind::Entity,
            SymbolKind::Error,
            SymbolKind::Aggregate,
        ];

        let Some(found) =
            self.checker
                .expect_kind(&self.scope, path, &kinds, "a value, entity or error")
        else {
            return Type::Unknown;
        };

        let found = match self.checker.generic_value(&found) {
            Some(_) => match self.generic_record(&found, fields, expected) {
                Some(instance) => instance,
                None => return Type::Unknown,
            },
            None => found,
        };

        let declared = self.checker.fields_of(&found).unwrap_or(&[]);

        let names: Vec<String> = (declared.iter())
            .filter(|field| field.value.is_none())
            .map(|field| field.name.name.clone())
            .collect();

        for name in &names {
            if !fields.iter().any(|given| &given.name.name == name) {
                self.error(
                    format!("`{}` needs a value for `{name}`", path.text()),
                    path.span(),
                );
            }
        }

        if found.kind == SymbolKind::Aggregate && !self.makes(&found) {
            self.error(self.made_by_factories(&found), path.span());
        }

        let ty = Type::Named(found);
        self.field_values(&ty, fields);

        ty
    }

    /// Whether this body may build `aggregate` whole: one of its own
    /// factories, or an adapter that keeps it for its repository.
    fn makes(&self, aggregate: &Resolved) -> bool {
        match self.kind {
            BodyKind::Factory(decl) => {
                decl.name.name == aggregate.name
                    && self.scope.context.name.name == aggregate.context
            }
            BodyKind::Adapter(adapter) => {
                let kept = self
                    .checker
                    .find_path(&self.scope, &adapter.port)
                    .filter(|port| port.kind == SymbolKind::Repository)
                    .and_then(|repository| self.checker.repository_aggregate(&repository));

                kept.as_ref() == Some(aggregate)
            }
            _ => false,
        }
    }

    /// Where a new aggregate comes from instead: its factories, by name.
    fn made_by_factories(&self, aggregate: &Resolved) -> String {
        let scope = self.checker.scope_of(aggregate);

        let factories: Vec<String> = (scope.context.aggregates.iter())
            .filter(|a| a.name.name == aggregate.name)
            .flat_map(|a| &a.factories)
            .map(|f| format!("`{}.{}`", aggregate.name, f.name.name))
            .collect();

        let name = &aggregate.name;

        match factories[..] {
            [] => format!(
                "a new `{name}` is made by a factory; declare one in the aggregate: \
                 `factory Open(...) = {name} {{ ... }}`"
            ),
            [ref only] => format!("a new `{name}` is made by its factory: {only}"),
            _ => format!(
                "a new `{name}` is made by one of its factories: {}",
                factories.join(", ")
            ),
        }
    }

    /// `start with { x: 11 }`: a copy with some fields replaced.
    /// An aggregate is only copied this way inside its own commands and
    /// factories; elsewhere it changes by issuing a command.
    pub(super) fn with(&mut self, base: &Expr, fields: &[FieldValue]) -> Type {
        let ty = self.expr(base, None);

        if let Type::Named(found) = &ty {
            let own_command = matches!(
                self.kind,
                BodyKind::Command(decl)
                    if decl.name.name == found.name && self.scope.context.name.name == found.context
            );

            if found.kind == SymbolKind::Aggregate && !own_command && !self.makes(found) {
                let message = format!(
                    "a `{}` changes by issuing one of its commands, not with `with`",
                    found.name
                );

                self.error(message, base.span);
            }
        }

        self.field_values(&ty, fields);
        ty
    }

    /// The instance a generic record makes: the one expected, or the one its
    /// field values' types call for.
    fn generic_record(
        &mut self,
        template: &Resolved,
        fields: &[FieldValue],
        expected: Option<&Type>,
    ) -> Option<Resolved> {
        if let Some(Type::Named(instance)) = expected {
            if self
                .checker
                .is_instance_of(&Type::Named(instance.clone()), template)
            {
                return Some(instance.clone());
            }
        }

        let declaration = self.checker.generic_value(template)?;
        let mut bound = HashMap::new();
        let template_scope = self.checker.scope_of(template);

        for given in fields {
            let Some(value) = &given.value else { continue };
            let found = self.expr(value, None);

            let field = declaration
                .fields
                .iter()
                .find(|f| f.name.name == given.name.name);

            if let Some(field) = field {
                let parameters = &declaration.type_parameters;
                self.checker
                    .match_type(&template_scope, &field.ty, &found, parameters, &mut bound);
            }
        }

        let span = fields
            .first()
            .map_or(croft_syntax::Span::default(), |f| f.name.span);

        let arguments =
            self.type_arguments(&declaration.type_parameters, &bound, &template.name, span)?;

        self.checker.instance(&self.scope, template, arguments)
    }
}
