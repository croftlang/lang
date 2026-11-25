//! `main`: sets up the program, runs the domain (or the target's `main`
//! service), and handles the events it raised. If it fails, the program
//! prints the error's name and stops with exit code 1.

use dn_analysis::Type;

use crate::names::{c_name, c_string, error_code};
use crate::Generator;

impl<'a> Generator<'a> {
    pub fn main_function(&self) -> String {
        let context = &self.program.contexts[&self.start.context];
        let service = context
            .functions
            .iter()
            .find(|f| f.name.name == self.start.name);
        let result = service.map_or(Type::Unit, |s| self.function_result(&self.start.context, s));
        let failure = if matches!(result, Type::Result(..)) {
            "    if (!result.ok) {\n        dn_fail(dn_error_name(result.error.code));\n    }\n"
        } else {
            ""
        };
        format!(
            "int main(int argc, char **argv)\n{{\n    {} result;\n\n    dn_arguments_set(argc, argv);\n    dn_init();\n    \
             {}(&result);\n    dn_events_dispatch_from(0);\n{failure}    return 0;\n}}\n",
            self.c_type(&result),
            c_name(&self.start)
        )
    }

    /// `dn_error_name`, which gives each error's name for messages.
    pub fn error_names(&self, prototypes: &mut Vec<String>) -> String {
        let signature = "const char *dn_error_name(dn_u16 code)";
        prototypes.push(format!("{signature};"));
        let mut body = String::new();
        for (error, _) in self.errors() {
            let name = format!("{}.{}", error.context, error.name);
            body.push_str(&format!(
                "    if (code == {}) {{\n        return {};\n    }}\n",
                error_code(&error),
                c_string(&name)
            ));
        }
        format!("{signature}\n{{\n{body}    (void) code;\n    return \"an unknown error\";\n}}\n\n")
    }
}
