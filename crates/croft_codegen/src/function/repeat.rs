//! Loops: `loop { ... }` until `break`, or once for each value of a counter.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind, LoopCounter};

use super::range::Span;
use super::Writer;
use crate::types::width::width_for;

impl<'g, 'a> Writer<'g, 'a> {
    /// `loop { ... }`, with a counter when it has one.
    pub fn repeat(&mut self, counter: Option<&LoopCounter>, block: &Expr) -> String {
        let ExprKind::Block(statements) = &block.kind else {
            return "0".to_string();
        };

        self.push_scope();

        match counter {
            Some(counter) => {
                let range = counter
                    .constraints
                    .iter()
                    .find(|r| r.name.text() == "Range");

                let arguments = range.map_or(&[][..], |r| &r.arguments[..]);

                let bounds: Vec<String> = (arguments.iter())
                    .map(|a| self.value(a, &Type::Int))
                    .collect();

                let base = format!("v_{}", counter.name.name);

                // When its bounds are known the counter is kept in the
                // smallest type that holds them, and one past the last, which
                // ends the loop.
                let known = match arguments {
                    [low, high] => self.range_of(low).zip(self.range_of(high)),
                    _ => None,
                };

                let variable = match known {
                    Some((low, high)) => {
                        let variable = self
                            .raw_variable(&base, width_for(low.low, high.high.saturating_add(1)));

                        self.note_range(&variable, Span::new(low.low, high.high));
                        variable
                    }
                    None => self.variable(&base, &Type::Int),
                };

                let (low, high) = (
                    bounds.first().cloned().unwrap_or_default(),
                    bounds.get(1).cloned().unwrap_or_default(),
                );

                self.open(format!(
                    "for ({variable} = {low}; {variable} <= {high}; {variable}++)"
                ));

                self.bind(&counter.name.name, variable, Type::Int);
            }
            None => self.open("for (;;)"),
        }

        self.block(statements, &Type::Unit);
        self.close();
        self.pop_scope();

        "0".to_string()
    }
}
