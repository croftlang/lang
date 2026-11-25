//! Which types are accepted where another is expected.

use super::{Checker, Resolved};
use crate::symbols::SymbolKind;
use crate::types::Type;

impl<'a> Checker<'a> {
    /// The errors a type stands for: an error is itself, `Room.Error` is every
    /// error of `Room`, and an enum is every error it includes.
    pub(super) fn error_members(&self, named: &Resolved) -> Vec<Resolved> {
        let context = &self.program.contexts[&named.context];
        match named.kind {
            SymbolKind::Error => vec![named.clone()],
            SymbolKind::ErrorSet => {
                let aggregate = named.name.trim_end_matches(".Error");
                let errors = context
                    .aggregates
                    .iter()
                    .filter(|a| a.name.name == aggregate);
                errors
                    .flat_map(|a| &a.errors)
                    .filter_map(|error| {
                        self.lookup(&named.context, &format!("{aggregate}.{}", error.name.name))
                    })
                    .collect()
            }
            SymbolKind::Enum => {
                let scope = super::Scope {
                    context,
                    aggregate: None,
                };
                let decl = context.enums.iter().find(|e| e.name.name == named.name);
                let includes = decl.into_iter().flat_map(|decl| &decl.includes);
                includes
                    .filter_map(|include| self.resolve(&scope, include))
                    .flat_map(|included| self.error_members(&included))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Whether a value of type `from` can be used where `to` is expected.
    /// A plain value is also accepted where a `Result` of it is expected.
    pub(super) fn accepts(&self, to: &Type, from: &Type) -> bool {
        match (to, from) {
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (_, Type::Errors(errors)) if !errors.is_empty() => {
                errors.iter().all(|error| self.accepts(to, error))
            }
            // `Errors([])` is any error at all, which a domain's body can pass up.
            (Type::Errors(errors), from) if errors.is_empty() => match from {
                Type::Named(named) => !self.error_members(named).is_empty(),
                Type::Errors(_) => true,
                _ => false,
            },
            // One of several errors takes any error one of them takes.
            (Type::Errors(errors), from) => errors.iter().any(|error| self.accepts(error, from)),
            // A whole number is a `Decimal` with nothing after the point. Inside a
            // list or option it would need every item converted, so it isn't taken there.
            (Type::Decimal(_), Type::Int) => true,
            (Type::List(to), Type::List(from)) | (Type::Option(to), Type::Option(from)) => {
                self.accepts(to, from) && !matches!((&**to, &**from), (Type::Decimal(_), Type::Int))
            }
            (Type::Result(to_value, to_error), Type::Result(from_value, from_error)) => {
                self.accepts(to_value, from_value) && self.accepts(to_error, from_error)
            }
            (Type::Result(to_value, _), _) if self.accepts(to_value, from) => true,
            (Type::Named(to), Type::Named(from)) => to == from || self.includes_errors(to, from),
            _ => to == from,
        }
    }

    /// Whether every error `from` stands for is one of the errors `to` stands for.
    fn includes_errors(&self, to: &Resolved, from: &Resolved) -> bool {
        let members = self.error_members(to);
        let from_members = self.error_members(from);
        !from_members.is_empty() && from_members.iter().all(|error| members.contains(error))
    }
}
