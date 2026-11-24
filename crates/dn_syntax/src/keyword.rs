//! Reserved words. Words such as `on`, `for`, `in`, `from`, `when`, `start`
//! and `target` are only special in one place, so they stay identifiers and
//! the parser checks for them where they are expected.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Context,
    Publish,
    Use,
    Value,
    Entity,
    Aggregate,
    Identity,
    Enum,
    Const,
    Constraint,
    Invariant,
    Error,
    Event,
    Command,
    Policy,
    Fn,
    Service,
    Port,
    Adapter,
    Repository,
    Translation,
    Application,
    Let,
    Set,
    Emit,
    Issue,
    If,
    Match,
    With,
    Where,
    Each,
    Uses,
    Native,
    Or,
    True,
    False,
}

const KEYWORDS: [(&str, Keyword); 36] = [
    ("context", Keyword::Context),
    ("publish", Keyword::Publish),
    ("use", Keyword::Use),
    ("value", Keyword::Value),
    ("entity", Keyword::Entity),
    ("aggregate", Keyword::Aggregate),
    ("identity", Keyword::Identity),
    ("enum", Keyword::Enum),
    ("const", Keyword::Const),
    ("constraint", Keyword::Constraint),
    ("invariant", Keyword::Invariant),
    ("error", Keyword::Error),
    ("event", Keyword::Event),
    ("command", Keyword::Command),
    ("policy", Keyword::Policy),
    ("fn", Keyword::Fn),
    ("service", Keyword::Service),
    ("port", Keyword::Port),
    ("adapter", Keyword::Adapter),
    ("repository", Keyword::Repository),
    ("translation", Keyword::Translation),
    ("application", Keyword::Application),
    ("let", Keyword::Let),
    ("set", Keyword::Set),
    ("emit", Keyword::Emit),
    ("issue", Keyword::Issue),
    ("if", Keyword::If),
    ("match", Keyword::Match),
    ("with", Keyword::With),
    ("where", Keyword::Where),
    ("each", Keyword::Each),
    ("uses", Keyword::Uses),
    ("native", Keyword::Native),
    ("or", Keyword::Or),
    ("true", Keyword::True),
    ("false", Keyword::False),
];

impl Keyword {
    pub fn from_word(word: &str) -> Option<Keyword> {
        KEYWORDS
            .iter()
            .find(|(text, _)| *text == word)
            .map(|(_, keyword)| *keyword)
    }

    pub fn as_str(self) -> &'static str {
        KEYWORDS
            .iter()
            .find(|(_, keyword)| *keyword == self)
            .map(|(text, _)| *text)
            .unwrap_or("?")
    }
}
