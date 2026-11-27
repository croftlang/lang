//! Reserved words. Words such as `on`, `for`, `from`, `when`, `as`, `include`
//! and `contexts` are only special in one place, so they stay identifiers and
//! the parser checks for them where they are expected.

/// A reserved word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Context,
    Domain,
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
    Factory,
    Policy,
    Fn,
    Service,
    Port,
    Adapter,
    Repository,
    Translation,
    Let,
    Set,
    Loop,
    Break,
    Emit,
    Issue,
    If,
    Match,
    With,
    Where,
    Each,
    Calls,
    Native,
    Or,
    True,
    False,
}

/// Each reserved word as written, and the keyword it reads as.
const KEYWORDS: [(&str, Keyword); 39] = [
    ("context", Keyword::Context),
    ("domain", Keyword::Domain),
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
    ("factory", Keyword::Factory),
    ("policy", Keyword::Policy),
    ("fn", Keyword::Fn),
    ("service", Keyword::Service),
    ("port", Keyword::Port),
    ("adapter", Keyword::Adapter),
    ("repository", Keyword::Repository),
    ("translation", Keyword::Translation),
    ("let", Keyword::Let),
    ("loop", Keyword::Loop),
    ("break", Keyword::Break),
    ("set", Keyword::Set),
    ("emit", Keyword::Emit),
    ("issue", Keyword::Issue),
    ("if", Keyword::If),
    ("match", Keyword::Match),
    ("with", Keyword::With),
    ("where", Keyword::Where),
    ("each", Keyword::Each),
    ("calls", Keyword::Calls),
    ("native", Keyword::Native),
    ("or", Keyword::Or),
    ("true", Keyword::True),
    ("false", Keyword::False),
];

impl Keyword {
    /// The keyword a word reads as, if it is reserved.
    pub fn from_word(word: &str) -> Option<Keyword> {
        KEYWORDS
            .iter()
            .find(|(text, _)| *text == word)
            .map(|(_, keyword)| *keyword)
    }

    /// Every reserved word.
    pub fn words() -> impl Iterator<Item = &'static str> {
        KEYWORDS.iter().map(|(text, _)| *text)
    }

    /// The keyword as written.
    pub fn as_str(self) -> &'static str {
        KEYWORDS
            .iter()
            .find(|(_, keyword)| *keyword == self)
            .map(|(text, _)| *text)
            .unwrap_or("?")
    }
}
