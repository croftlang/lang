//! Matching call arguments to parameters.

use b_syntax::ast::Argument;
use b_syntax::Span;

use super::BodyChecker;
use crate::types::Type;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// Matches arguments, positional or named, to parameters, each exactly once.
    pub(super) fn check_arguments(
        &mut self,
        arguments: &[Argument],
        parameters: &[(String, Type)],
        callee: &str,
        span: Span,
        require_names: bool,
    ) {
        let mut given = vec![false; parameters.len()];
        let mut position = 0;
        for argument in arguments {
            let index = match &argument.name {
                Some(name) => parameters
                    .iter()
                    .position(|(parameter, _)| parameter == &name.name),
                None => {
                    if require_names {
                        let message =
                            format!("arguments to `{callee}` are always named: `name: value`");
                        self.error(message, argument.value.span);
                    }
                    position += 1;
                    (position <= parameters.len()).then_some(position - 1)
                }
            };
            let Some(index) = index else {
                if let Some(name) = &argument.name {
                    self.error(
                        format!("`{callee}` has no parameter `{}`", name.name),
                        name.span,
                    );
                } else {
                    let message = format!(
                        "`{callee}` takes {} arguments, found {}",
                        parameters.len(),
                        arguments.len()
                    );
                    self.error(message, argument.value.span);
                }
                self.expr(&argument.value, None);
                continue;
            };
            if given[index] {
                self.error(
                    format!("`{}` is given more than once", parameters[index].0),
                    argument.value.span,
                );
            }
            given[index] = true;
            self.expect(&argument.value, &parameters[index].1);
        }
        for (index, (name, _)) in parameters.iter().enumerate() {
            if !given[index] {
                self.error(format!("`{callee}` needs a value for `{name}`"), span);
            }
        }
    }
}
