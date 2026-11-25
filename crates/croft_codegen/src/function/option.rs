//! The built-in methods of options, and calling a lambda argument.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, ExprKind};

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
        if name != "map" {
            return self.unwrap_or(option, item, arguments, ty);
        }

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
                return self.unwrap_or(option, item, arguments, ty);
            }
        }

        temp
    }

    /// `option.unwrap_or(fallback)`: a value with room for the strings of either.
    fn unwrap_or(
        &mut self,
        option: &str,
        item: &Type,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let Some(argument) = arguments.first() else {
            return "0".to_string();
        };

        // A literal is only written out when it is needed.
        if let ExprKind::String(text) = &argument.value.kind {
            let literal = self.gen.text_type(text.chars().count() as i64);
            let ty = self.gen.keep_bound(&self.gen.wider(item, &literal), ty);
            let temp = self.temp(&ty);

            self.open(format!("if ({option}.some)"));
            self.copy_into(&temp, &format!("{option}.value"), item, &ty);
            self.close();

            self.open("else");
            self.assign(&temp, &argument.value, &ty);
            self.close();

            return temp;
        }

        let (fallback, fallback_type) = self.eval(&argument.value, Some(ty));
        let ty = self
            .gen
            .keep_bound(&self.gen.wider(item, &fallback_type), ty);

        // The fallback as a value of the result's type, but for its bounds.
        let given = self.gen.keep_bound(&fallback_type, &ty);
        let fallback = self.convert(fallback, &fallback_type, &given);
        let temp = self.temp(&ty);

        if self.gen.c_type(item) == self.gen.c_type(&ty)
            && self.gen.c_type(&given) == self.gen.c_type(&ty)
        {
            self.line(format!(
                "{temp} = {option}.some ? {option}.value : {fallback};"
            ));

            return temp;
        }

        self.open(format!("if ({option}.some)"));
        self.copy_into(&temp, &format!("{option}.value"), item, &ty);
        self.close();

        self.open("else");
        self.copy_into(&temp, &fallback, &given, &ty);
        self.close();

        temp
    }
}
