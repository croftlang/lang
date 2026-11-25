//! Looking up what generated code needs to know about declarations: their
//! parameters, results, identities and event keys.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;

use crate::Generator;

impl<'a> Generator<'a> {
    /// Parameters of a function, translation, service, constraint or factory
    /// declared in Croft.
    pub fn parameters_of(&self, found: &Resolved) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&found.context];

        if found.kind == SymbolKind::Factory {
            let Some((aggregate, factory)) = found.name.split_once('.') else {
                return Vec::new();
            };

            let aggregate = Resolved {
                context: found.context.clone(),
                name: aggregate.to_string(),
                kind: SymbolKind::Aggregate,
            };

            return self.factory_parameters(&aggregate, factory);
        }

        let parameters = if found.kind == SymbolKind::Constraint {
            context
                .constraints
                .iter()
                .find(|c| c.name.name == found.name)
                .map(|c| c.parameters.as_slice())
        } else {
            context
                .functions
                .iter()
                .find(|f| f.name.name == found.name)
                .and_then(|f| f.parameters.as_deref())
        };

        self.lower_parameters(&found.context, None, parameters.unwrap_or(&[]))
    }

    pub fn operation_parameters(&self, port: &Resolved, operation: &str) -> Vec<(String, Type)> {
        let operations = self.model.operations_of(port);
        let operation = operations.into_iter().find(|op| op.name == operation);

        (operation.into_iter().flat_map(|op| op.parameters))
            .map(|(name, ty, _)| (name, ty))
            .collect()
    }

    pub fn command_parameters(&self, aggregate: &Resolved, command: &str) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&aggregate.context];

        let decl = context
            .aggregates
            .iter()
            .find(|a| a.name.name == aggregate.name);

        let command = decl.and_then(|a| a.commands.iter().find(|c| c.name.name == command));
        let parameters = command.and_then(|c| c.parameters.as_deref()).unwrap_or(&[]);

        self.lower_parameters(&aggregate.context, Some(&aggregate.name), parameters)
    }

    pub fn factory_parameters(&self, aggregate: &Resolved, factory: &str) -> Vec<(String, Type)> {
        let context = &self.program.contexts[&aggregate.context];

        let decl = context
            .aggregates
            .iter()
            .find(|a| a.name.name == aggregate.name);

        let factory = decl.and_then(|a| a.factories.iter().find(|f| f.name.name == factory));
        let parameters = factory.and_then(|f| f.parameters.as_deref()).unwrap_or(&[]);

        self.lower_parameters(&aggregate.context, Some(&aggregate.name), parameters)
    }

    /// What a function gives back. A domain's body can fail with any error.
    pub fn function_result(
        &self,
        context: &str,
        function: &croft_syntax::ast::FunctionDecl,
    ) -> Type {
        match &function.return_type {
            Some(ty) => self.model.lower_type(context, None, ty),
            None if function.kind == croft_syntax::ast::FunctionKind::Domain => {
                Type::result(Type::Unit, Type::Errors(Vec::new()))
            }
            None => Type::Unit,
        }
    }

    /// What a queued event is filed under, so saving its aggregate finds it:
    /// the identity, when it is a number. Text identities share one key.
    pub fn event_key(&self, aggregate: &Resolved, identity: &str) -> String {
        let is_number = match self.model.identity_of(aggregate) {
            Type::Named(named) => {
                let context = &self.program.contexts[&named.context];

                context
                    .identities
                    .iter()
                    .any(|i| i.name.name == named.name && i.ty.path.text() == "Int")
            }
            other => other == Type::Int,
        };

        if is_number {
            format!("(croft_int) {identity}")
        } else {
            "0".to_string()
        }
    }

    /// The name of an aggregate's identity field.
    pub fn identity_field(&self, aggregate: &Resolved) -> String {
        let fields = self.model.fields(aggregate);

        fields
            .into_iter()
            .find(|(_, _, f)| f.is_identity)
            .map_or("id".to_string(), |(name, _, _)| name)
    }
}
