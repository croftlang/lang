//! Putting call arguments in parameter order, and passing them the C way.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// The argument for each parameter, by name or position.
    pub fn ordered<'e>(
        &self,
        arguments: &'e [Argument],
        parameters: &[String],
    ) -> Vec<Option<&'e Expr>> {
        let mut ordered = vec![None; parameters.len()];
        let mut position = 0;
        for argument in arguments {
            let index = match &argument.name {
                Some(name) => parameters.iter().position(|p| p == &name.name),
                None => {
                    position += 1;
                    Some(position - 1)
                }
            };
            if let Some(slot) = index.and_then(|index| ordered.get_mut(index)) {
                *slot = Some(&argument.value);
            }
        }
        ordered
    }

    /// Works out each argument for parameters of the given types, and passes
    /// it: numbers by value, everything else by pointer.
    pub fn pass_all(
        &mut self,
        arguments: &[Argument],
        parameters: &[(String, Type)],
    ) -> Vec<String> {
        let names: Vec<String> = parameters.iter().map(|(name, _)| name.clone()).collect();
        let values = self.ordered(arguments, &names);
        let mut passed = Vec::new();
        for ((_, ty), value) in parameters.iter().zip(values) {
            let c = match value {
                Some(value) => self.value(value, ty),
                None => "0".to_string(),
            };
            passed.push(self.pass(c, ty));
        }
        passed
    }

    pub fn pass(&self, c: String, ty: &Type) -> String {
        if self.gen.is_scalar(ty) {
            return c;
        }
        match c
            .strip_prefix("(*")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            Some(pointer) if !pointer.contains(['(', ')', '.', ' ']) => pointer.to_string(),
            _ => format!("&{c}"),
        }
    }

    /// Calls `function` with the arguments and a temporary for the result.
    pub fn call_into(&mut self, function: &str, mut passed: Vec<String>, result: &Type) -> String {
        let temp = self.temp(result);
        passed.push(format!("&{temp}"));
        self.line(format!("{function}({});", passed.join(", ")));
        temp
    }
}
