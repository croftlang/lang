//! The kinds of thing a name can refer to.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Value,
    Entity,
    Aggregate,
    Enum,
    Identity,
    Error,
    ErrorSet,
    Event,
    Command,
    Constraint,
    Const,
    Function,
    Service,
    Translation,
    Policy,
    Port,
    Adapter,
    Repository,
}

impl SymbolKind {
    pub fn is_type(self) -> bool {
        use SymbolKind::*;
        matches!(
            self,
            Value | Entity | Aggregate | Enum | Identity | Error | ErrorSet | Event
        )
    }

    pub fn describe(self) -> &'static str {
        use SymbolKind::*;
        match self {
            Value => "a value",
            Entity => "an entity",
            Aggregate => "an aggregate",
            Enum => "an enum",
            Identity => "an identity",
            Error => "an error",
            ErrorSet => "an aggregate's errors",
            Event => "an event",
            Command => "a command",
            Constraint => "a constraint",
            Const => "a constant",
            Function => "a function",
            Service => "a service",
            Translation => "a translation",
            Policy => "a policy",
            Port => "a port",
            Adapter => "an adapter",
            Repository => "a repository",
        }
    }
}
