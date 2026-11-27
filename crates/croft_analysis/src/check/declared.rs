//! What declared types are made of: their fields, variants and errors.

use croft_syntax::ast::{Field, Invariant, TypeExpr};

use super::{Checker, Resolved, Scope};
use crate::symbols::SymbolKind;
use crate::types::{Type, MAX_PLACES};

impl<'a> Checker<'a> {
    /// The type a written type stands for. Names that don't resolve were
    /// already reported, so they become `Unknown`.
    pub(super) fn lower_type(&self, scope: &Scope, ty: &TypeExpr) -> Type {
        let argument = |index: usize| {
            ty.arguments
                .get(index)
                .map_or(Type::Unknown, |argument| self.lower_type(scope, argument))
        };

        match ty.path.text().as_str() {
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Decimal" => Type::Decimal(ty.number.unwrap_or(0).clamp(0, MAX_PLACES) as u32),
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Bytes" => Type::Bytes,
            "Unit" => Type::Unit,
            "List" => Type::list(argument(0)),
            "Map" => Type::map(argument(0), argument(1)),
            "Set" => Type::set(argument(0)),
            "Option" => Type::option(argument(0)),
            "Result" => Type::result(argument(0), argument(1)),
            _ => match self.resolve(scope, &ty.path) {
                Some(found) if self.generic_value(&found).is_some() => {
                    let instance = self.instance(scope, &found, ty.arguments.clone());
                    instance.map_or(Type::Unknown, Type::Named)
                }
                Some(found) if found.kind.is_type() => Type::Named(found),
                _ => Type::Unknown,
            },
        }
    }

    /// Where a declaration's own names are looked up: its context, and its
    /// aggregate if it belongs to one.
    pub(super) fn scope_of(&self, named: &Resolved) -> Scope<'a> {
        let context = &self.program.contexts[&named.context];

        let aggregate = match named.name.split_once('.') {
            Some((aggregate, _)) => Some(aggregate),
            None if named.kind == SymbolKind::Aggregate => Some(named.name.as_str()),
            None => None,
        };

        let aggregate = aggregate.and_then(|name| {
            context
                .aggregates
                .iter()
                .find(|a| a.name.name == name)
                .map(|a| a.name.name.as_str())
        });

        Scope { context, aggregate }
    }

    /// The fields of a value, entity, aggregate, error or event.
    pub(super) fn fields_of(&self, named: &Resolved) -> Option<&'a [Field]> {
        let context = &self.program.contexts[&named.context];

        let (aggregate, member) = match named.name.split_once('.') {
            Some((aggregate, member)) => (
                context.aggregates.iter().find(|a| a.name.name == aggregate),
                member,
            ),
            None => (None, named.name.as_str()),
        };

        let fields = match (named.kind, aggregate) {
            (SymbolKind::Value, None) => {
                &context
                    .values
                    .iter()
                    .find(|v| v.name.name == member)?
                    .fields
            }
            (SymbolKind::Error, None) => {
                &context
                    .errors
                    .iter()
                    .find(|e| e.name.name == member)?
                    .fields
            }
            (SymbolKind::Aggregate, _) => {
                &context
                    .aggregates
                    .iter()
                    .find(|a| a.name.name == member)?
                    .fields
            }
            (SymbolKind::Entity, Some(a)) => {
                &a.entities.iter().find(|e| e.name.name == member)?.fields
            }
            (SymbolKind::Error, Some(a)) => {
                &a.errors.iter().find(|e| e.name.name == member)?.fields
            }
            (SymbolKind::Event, Some(a)) => {
                &a.events.iter().find(|e| e.name.name == member)?.fields
            }
            _ => return None,
        };

        Some(fields)
    }

    /// The invariants of a value declared in its context, which every one
    /// the program makes must keep. Other types have none kept this way.
    pub(super) fn value_invariants(&self, named: &Resolved) -> &'a [Invariant] {
        if named.kind != SymbolKind::Value {
            return &[];
        }

        let context = &self.program.contexts[&named.context];

        (context.values.iter())
            .find(|value| value.name.name == named.name)
            .map_or(&[], |value| value.invariants.as_slice())
    }

    /// The type of one field of a named type.
    pub(super) fn field_type(&self, named: &Resolved, field: &str) -> Option<Type> {
        let fields = self.fields_of(named)?;
        let field = fields.iter().find(|f| f.name.name == field)?;
        Some(self.lower_type(&self.scope_of(named), &field.ty))
    }
}

impl<'a> Checker<'a> {}
