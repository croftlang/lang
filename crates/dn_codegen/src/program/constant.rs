//! Constants: a `#define` when the value is a number known now, otherwise a
//! global set once by `dn_init` when the program starts.

use dn_analysis::symbols::SymbolKind;

use crate::function::{Returns, Writer};
use crate::names::c_name;
use crate::types::constant::evaluate;
use crate::Generator;

impl<'a> Generator<'a> {
    /// The `#define`s for the header, the globals, and `dn_init`.
    pub fn constants(&self) -> (String, String, String) {
        let (mut defines, mut globals) = (String::new(), String::new());
        let mut writer = Writer::new(self, "", Returns::Nothing);
        for found in self.model.all(SymbolKind::Const) {
            let context = &self.program.contexts[&found.context];
            let Some(constant) = context.consts.iter().find(|c| c.name.name == found.name) else {
                continue;
            };
            let ty = self.model.lower_type(&found.context, None, &constant.ty);
            let name = c_name(&found);
            if self.is_scalar(&ty) {
                if let Some(value) =
                    evaluate(self.program, &self.model, &found.context, &constant.value)
                {
                    defines.push_str(&format!("#define {name} ({value}L)\n"));
                    continue;
                }
            }
            globals.push_str(&format!("{} {name};\n", self.c_type(&ty)));
            writer.context = found.context.clone();
            let value = writer.value(&constant.value, &ty);
            writer.line(format!("{name} = {value};"));
        }
        if !defines.is_empty() {
            defines.push('\n');
        }
        (defines, globals, writer.finish("void dn_init(void)"))
    }
}
