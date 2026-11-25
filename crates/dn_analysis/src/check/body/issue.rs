//! Issuing commands, raising events, native calls, and failing with `?` and `or`.

use dn_syntax::ast::{Argument, Expr, ExprId, ExprKind, Identifier, Issue};
use dn_syntax::Span;

use super::{BodyChecker, BodyKind};
use crate::check::data::expr_path;
use crate::facts::Meaning;
use crate::symbols::SymbolKind;
use crate::types::{Resolved, Type};

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `issue Cmd(...) on room` gives back the changed room. `issue Cmd(...)
    /// on Room(id)` loads, changes and saves it, so only services and policies can.
    pub(super) fn issue(&mut self, id: ExprId, issue: &Issue, span: Span) -> Type {
        if matches!(
            self.kind,
            BodyKind::Command(_) | BodyKind::Constraint | BodyKind::Constant
        ) {
            self.error("only services, policies and functions can issue commands; a command changes one aggregate", span);
        }
        let (aggregate, by_identity) = match self.loaded_aggregate(&issue.target) {
            Some(aggregate) => (Some(aggregate), true),
            None => match self.expr(&issue.target, None) {
                Type::Named(named) if named.kind == SymbolKind::Aggregate => (Some(named), false),
                Type::Unknown => (None, false),
                other => {
                    self.error(
                        format!("commands are issued on an aggregate, not on `{other}`"),
                        issue.target.span,
                    );
                    (None, false)
                }
            },
        };
        let Some(aggregate) = aggregate else {
            for argument in &issue.arguments {
                self.expr(&argument.value, None);
            }
            return Type::Unknown;
        };
        let meaning = if by_identity {
            Meaning::IssueByIdentity(aggregate.clone())
        } else {
            Meaning::IssueOnValue(aggregate.clone())
        };
        self.note(Some(id), meaning);
        self.issue_arguments(&aggregate, &issue.command, &issue.arguments);
        let errors = self
            .checker
            .lookup(&aggregate.context, &format!("{}.Error", aggregate.name));
        let errors = errors.map_or(Type::Unknown, Type::Named);
        if !by_identity {
            return Type::result(Type::Named(aggregate), errors);
        }
        let storage = self.checker.storage_error();
        Type::result(Type::Unit, Type::Errors(vec![errors, storage]))
    }

    /// The aggregate named by an `on Room(id)` target, after checking `id`.
    pub(super) fn loaded_aggregate(&mut self, target: &Expr) -> Option<Resolved> {
        let ExprKind::Call(callee, arguments) = &target.kind else {
            return None;
        };
        let path = expr_path(callee)?;
        let aggregate = self
            .checker
            .resolve(&self.scope, &path)
            .filter(|f| f.kind == SymbolKind::Aggregate)?;
        if !matches!(self.kind, BodyKind::Service | BodyKind::Policy) {
            self.error(
                "only services and policies can load an aggregate by its identity",
                target.span,
            );
        }
        if self.checker.repository_for(&aggregate).is_none() {
            self.error(
                format!("`{}` has no repository to load it from", aggregate.name),
                target.span,
            );
        }
        let identity = self.checker.identity_of(&aggregate);
        let parameters = [("identity".to_string(), identity)];
        self.check_arguments(arguments, &parameters, &aggregate.name, target.span, false);
        Some(aggregate)
    }

    pub(super) fn issue_arguments(
        &mut self,
        aggregate: &Resolved,
        command: &Identifier,
        arguments: &[Argument],
    ) {
        let scope = self.checker.scope_of(aggregate);
        let decl = scope
            .context
            .aggregates
            .iter()
            .find(|a| a.name.name == aggregate.name);
        let Some(command_decl) =
            decl.and_then(|a| a.commands.iter().find(|c| c.name.name == command.name))
        else {
            self.error(
                format!("`{}` has no command `{}`", aggregate.name, command.name),
                command.span,
            );
            return;
        };
        let parameters: Vec<(String, Type)> = (command_decl.parameters.iter().flatten())
            .map(|p| (p.name.name.clone(), self.checker.parameter_type(&scope, p)))
            .collect();
        self.check_arguments(arguments, &parameters, &command.name, command.span, true);
    }
}
