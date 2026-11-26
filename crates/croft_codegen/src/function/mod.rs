//! Writing one C function. Croft bodies are expressions; C89 needs statements
//! and every variable declared first, so each value is worked out into a
//! temporary and the declarations are gathered at the top.

mod arguments;
mod block;
mod builtin;
mod bytes;
mod call;
mod constraint;
mod decimal;
mod expression;
mod failure;
mod issue;
mod list;
mod list_lambda;
mod map;
mod matching;
mod method;
mod operator;
mod option;
mod output;
mod property;
pub(crate) mod range;
mod record;
mod repeat;
mod scope;
mod string;
mod text;
mod unused;
mod view;

pub(crate) use unused::mentions;
mod usage;

use std::collections::{HashMap, HashSet};

use croft_analysis::Type;

use crate::Generator;

/// Where a value can be written straight to, rather than to a temporary of
/// its own: a place and its type.
pub type Into<'t> = Option<(&'t str, &'t Type)>;

/// How a function gives back its value or a failure.
pub enum Returns {
    /// `*out = value;` and failures as `out->ok = 0`.
    Value(Type),
    /// A command gives back its changed aggregate, or fails and undoes its events.
    Command,
    /// A policy gives nothing back; a failure just stops it.
    Nothing,
}

/// The fields a body can read and `set` without naming their owner.
pub struct OwnFields {
    pub prefix: String,
    pub fields: Vec<(String, Type)>,
    /// The aggregate whose fields these are, for a command; `None` for an adapter.
    pub owner: Option<croft_analysis::types::Resolved>,
}

pub struct Writer<'g, 'a> {
    pub gen: &'g Generator<'a>,
    pub context: String,
    pub aggregate: Option<String>,
    pub own: Option<OwnFields>,
    pub returns: Returns,
    /// Writing a factory, whose events learn their source once the body ends.
    pub factory: bool,
    /// The events a factory raised, to be given their source.
    pub emitted: Vec<croft_analysis::types::Resolved>,
    declarations: Vec<String>,
    lines: Vec<String>,
    depth: usize,
    names: HashSet<String>,
    /// C parameters, marked as used in `finish` if the body never reads them.
    parameters: Vec<String>,
    scopes: Vec<HashMap<String, (String, Type)>>,
    /// C names holding a `croft_string_view`: the function's string
    /// parameters, borrowed rather than copied.
    pub(crate) views: HashSet<String>,
    /// C names and places whose strings have a bound, and their types:
    /// `t_3` is a `List<Text(32)>`, `p_line` a `Text(32)`.
    pub(crate) bounded: HashMap<String, Type>,
    /// What the function was seen to give back: its result type, with the
    /// bounds of every value written to `out`.
    pub(crate) returned: Option<Type>,
    /// Temporaries that keep what was first written to them until the
    /// function returns, so a string can be borrowed from one.
    pub(crate) stable: HashSet<String>,
    /// Every temporary handed out, in order, with its C type.
    handed: Vec<(String, String)>,
    /// Temporaries nothing uses any more, by C type, to be handed out again.
    free: Vec<(String, String)>,
    /// C names holding a whole number whose values are known, such as a
    /// parameter with a `Range` or a `let` of a sum of two of them.
    pub(crate) ranges: HashMap<String, range::Span>,
}

impl<'g, 'a> Writer<'g, 'a> {
    pub fn new(gen: &'g Generator<'a>, context: &str, returns: Returns) -> Self {
        Writer {
            gen,
            context: context.to_string(),
            aggregate: None,
            own: None,
            returns,
            factory: false,
            emitted: Vec::new(),
            declarations: Vec::new(),
            lines: Vec::new(),
            depth: 1,
            names: HashSet::new(),
            parameters: Vec::new(),
            scopes: vec![HashMap::new()],
            views: HashSet::new(),
            bounded: HashMap::new(),
            returned: None,
            stable: HashSet::new(),
            handed: Vec::new(),
            free: Vec::new(),
            ranges: HashMap::new(),
        }
    }
}

/// The name an expression spells, if it is just a name: `ExitBlocked`, `Room.NotFound`.
pub fn path_of(value: &croft_syntax::ast::Expr) -> Option<croft_syntax::ast::Path> {
    use croft_syntax::ast::{ExprKind, Identifier, Path};

    match &value.kind {
        ExprKind::Name(name) => Some(Path {
            segments: vec![Identifier {
                name: name.clone(),
                span: value.span,
            }],
        }),
        ExprKind::Field(inner, field) => {
            let mut path = path_of(inner)?;
            path.segments.push(field.clone());
            Some(path)
        }
        _ => None,
    }
}
