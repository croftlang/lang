//! Loops: `loop { ... }` until `break`, or once for each value of a counter.

use croft_analysis::Type;
use croft_syntax::ast::{Expr, ExprKind, LoopCounter};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
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

                let bounds: Vec<String> = range.map_or(Vec::new(), |r| {
                    r.arguments
                        .iter()
                        .map(|a| self.value(a, &Type::Int))
                        .collect()
                });

                let variable = self.variable(&format!("v_{}", counter.name.name), &Type::Int);

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
