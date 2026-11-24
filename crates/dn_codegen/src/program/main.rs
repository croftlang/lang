//! `main`: sets up the program, runs the start service, and handles the
//! events it raised.

use dn_analysis::Type;

use crate::names::c_name;
use crate::Generator;

impl<'a> Generator<'a> {
    pub fn main_function(&self) -> String {
        let context = &self.program.contexts[&self.start.context];
        let service = context
            .functions
            .iter()
            .find(|f| f.name.name == self.start.name);
        let result = service.and_then(|s| s.return_type.as_ref());
        let result = result.map_or(Type::Unit, |ty| {
            self.model.lower_type(&self.start.context, None, ty)
        });
        let exit = if matches!(result, Type::Result(..)) {
            "result.ok ? 0 : 1"
        } else {
            "0"
        };
        format!(
            "int main(int argc, char **argv)\n{{\n    {} result;\n\n    dn_arguments_set(argc, argv);\n    dn_init();\n    \
             {}(&result);\n    dn_events_dispatch();\n    return {exit};\n}}\n",
            self.c_type(&result),
            c_name(&self.start)
        )
    }
}
