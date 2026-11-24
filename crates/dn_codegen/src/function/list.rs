//! The built-in methods of lists, written as loops. A list of entities is
//! also looked up, replaced and removed by identity.

use dn_analysis::symbols::SymbolKind;
use dn_analysis::Type;
use dn_syntax::ast::Argument;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn list_method(
        &mut self,
        list: &str,
        item: &Type,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let identity = self.identity_field(item);
        let key_type = match &identity {
            Some((_, key)) => key.clone(),
            None => Type::Int,
        };
        let first = |writer: &mut Self, want: &Type| match arguments.first() {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };
        let index = self.variable("index", &Type::Int);
        let each = format!("for ({index} = 0; {index} < (dn_int) {list}.length; {index}++)");
        let current = format!("{list}.items[{index}]");
        let temp = self.temp(ty);
        let matches = |writer: &mut Self, key: &str| match &identity {
            Some((field, _)) => format!("({current}.{field} == {key})"),
            None => writer.equal(&current, key, item),
        };
        match name {
            "contains" | "get" | "remove" if identity.is_some() || name == "contains" => {
                let key = first(self, if identity.is_some() { &key_type } else { item });
                let test = matches(self, &key);
                match name {
                    "contains" => self.line(format!("{temp} = 0;")),
                    "remove" => self.line(format!("{temp}.length = 0;")),
                    _ => self.line(format!("dn_clear(&{temp}, sizeof {temp});")),
                }
                self.open(&each);
                match name {
                    "contains" => self.line(format!("if ({test}) {temp} = 1;")),
                    "get" => self.line(format!("if ({test}) {temp} = {current};")),
                    _ => self.line(format!(
                        "if (!({test})) {temp}.items[{temp}.length++] = {current};"
                    )),
                }
                self.close();
            }
            "get" => {
                let position = first(self, &Type::Int);
                self.line(format!(
                    "{temp}.some = {position} >= 0 && {position} < (dn_int) {list}.length;"
                ));
                self.line(format!(
                    "if ({temp}.some) {temp}.value = {list}.items[{position}];"
                ));
            }
            "replace" => {
                let replacement = first(self, item);
                let field = identity.map_or_else(String::new, |(field, _)| field);
                self.line(format!("{temp} = {list};"));
                self.open(&each);
                self.line(format!("if ({current}.{field} == {replacement}.{field}) {temp}.items[{index}] = {replacement};"));
                self.close();
            }
            "add" => {
                let added = first(self, item);
                let capacity = self.gen.sizes.list_capacity(item);
                self.line(format!("{temp} = {list};"));
                self.line(format!(
                    "DN_CHECK({temp}.length < {capacity}, \"a list is full\");"
                ));
                self.line(format!(
                    "if ({temp}.length < {capacity}) {temp}.items[{temp}.length++] = {added};"
                ));
            }
            "drop" | "take" => {
                let count = first(self, &Type::Int);
                self.line(format!("{temp}.length = 0;"));
                self.open(&each);
                let keep = if name == "drop" { ">=" } else { "<" };
                self.line(format!(
                    "if ({index} {keep} {count}) {temp}.items[{temp}.length++] = {current};"
                ));
                self.close();
            }
            _ => self.list_lambda_method(list, item, name, arguments, ty, (&temp, &index)),
        }
        temp
    }

    /// The identity field of an entity, and its type.
    fn identity_field(&self, item: &Type) -> Option<(String, Type)> {
        let entity = item.named(&[SymbolKind::Entity])?;
        let fields = self.gen.model.fields(entity);
        let (name, ty, _) = fields.into_iter().find(|(_, _, field)| field.is_identity)?;
        Some((name, ty))
    }

    /// `[a, b, c]`: a list of exactly these items.
    pub fn list_literal(&mut self, items: &[dn_syntax::ast::Expr], ty: &Type) -> String {
        let Type::List(item) = ty else {
            return "0".to_string();
        };
        let values: Vec<String> = items.iter().map(|value| self.value(value, item)).collect();
        let list = self.temp(ty);
        let capacity = self.gen.sizes.list_capacity(item);
        self.line(format!(
            "{list}.length = {};",
            values.len().min(capacity as usize)
        ));
        if values.len() as i64 > capacity {
            self.line(format!(
                "DN_CHECK(0, \"a list has more than {capacity} items\");"
            ));
        }
        for (index, value) in values.iter().take(capacity as usize).enumerate() {
            self.line(format!("{list}.items[{index}] = {value};"));
        }
        list
    }
}
