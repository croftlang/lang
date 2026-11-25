//! The built-in methods of options, and calling a lambda argument.

use b_analysis::Type;
use b_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub(super) fn option_method(
        &mut self,
        option: &str,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let temp = self.temp(ty);
        match name {
            "map" => {
                let Type::Option(mapped) = ty else {
                    return temp;
                };
                self.line(format!("{temp}.some = {option}.some;"));
                self.open(format!("if ({option}.some)"));
                let value = self.lambda(arguments, format!("{option}.value"), item, mapped);
                self.line(format!("{temp}.value = {value};"));
                self.close();
            }
            _ => {
                let fallback = arguments
                    .first()
                    .map_or("0".to_string(), |a| self.value(&a.value, ty));
                self.line(format!(
                    "{temp} = {option}.some ? {option}.value : {fallback};"
                ));
            }
        }
        temp
    }
}
