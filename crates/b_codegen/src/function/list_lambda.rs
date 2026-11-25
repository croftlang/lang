//! List methods that call a lambda with each item.

use b_analysis::Type;
use b_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// Methods that call a lambda with each item.
    pub(super) fn list_lambda_method(
        &mut self,
        list: &str,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
        (temp, index): (&str, &str),
    ) {
        let current = format!("{list}.items[{index}]");
        let result = match (name, ty) {
            ("map", Type::List(mapped)) => (**mapped).clone(),
            ("sum" | "min_by" | "max_by", _) => Type::Int,
            _ => Type::Bool,
        };
        let best = self.variable("best", &Type::Int);
        match name {
            "all" => self.line(format!("{temp} = 1;")),
            "any" => self.line(format!("{temp} = 0;")),
            "count" | "sum" => self.line(format!("{temp} = 0;")),
            "find" => self.line(format!("{temp}.some = 0;")),
            "min_by" | "max_by" => {
                self.line(format!("{temp}.some = 0;"));
                self.line(format!("{best} = 0;"));
            }
            _ => self.line(format!("{temp}.length = 0;")),
        }
        self.open(format!(
            "for ({index} = 0; {index} < (b_int) {list}.length; {index}++)"
        ));
        let value = self.lambda(arguments, current.clone(), item, &result);
        match name {
            "all" => self.line(format!("if (!{value}) {temp} = 0;")),
            "any" => self.line(format!("if ({value}) {temp} = 1;")),
            "count" => self.line(format!("if ({value}) {temp}++;")),
            "sum" => self.line(format!("{temp} += {value};")),
            "find" => self.line(format!(
                "if (!{temp}.some && {value}) {{ {temp}.some = 1; {temp}.value = {current}; }}"
            )),
            "filter" => self.line(format!(
                "if ({value}) {temp}.items[{temp}.length++] = {current};"
            )),
            "map" => self.line(format!("{temp}.items[{temp}.length++] = {value};")),
            _ => {
                let better = if name == "min_by" { "<" } else { ">" };
                self.open(format!("if (!{temp}.some || {value} {better} {best})"));
                self.line(format!("{temp}.some = 1;"));
                self.line(format!("{temp}.value = {current};"));
                self.line(format!("{best} = {value};"));
                self.close();
            }
        }
        self.close();
    }
}
