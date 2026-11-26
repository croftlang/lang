//! The built-in methods of lists, written as loops. A list of entities is
//! also looked up, replaced and removed by identity.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::Argument;

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

        // The methods that give back the list's own items keep their bounds.
        let ty = match name {
            "get" | "find" | "min_by" | "max_by" => {
                self.gen.keep_bound(&Type::option(item.clone()), ty)
            }
            "remove" | "replace" | "drop" | "take" | "filter" => {
                self.gen.keep_bound(&Type::list(item.clone()), ty)
            }
            _ => ty.clone(),
        };

        let ty = &ty;

        if name == "add" {
            return self.add(list, item, arguments, ty);
        }

        let index = self.index_variable();
        let each = format!("for ({index} = 0; {index} < {list}.length; {index}++)");
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
                    _ => self.line(format!("croft_clear(&{temp}, sizeof {temp});")),
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
                    "{temp}.some = {position} >= 0 && {position} < (croft_int) {list}.length;"
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
            "drop" | "take" => {
                let count = first(self, &Type::Int);

                self.line(format!("{temp}.length = 0;"));
                self.open(&each);

                let keep = if name == "drop" { ">=" } else { "<" };

                // The counter is unsigned, so a count that may be below zero
                // is compared as a `croft_int`.
                let natural = (arguments.first())
                    .and_then(|argument| self.range_of(&argument.value))
                    .is_some_and(|range| range.is_natural());

                let at = if natural {
                    index.clone()
                } else {
                    format!("(croft_int) {index}")
                };

                self.line(format!(
                    "if ({at} {keep} {count}) {temp}.items[{temp}.length++] = {current};"
                ));

                self.close();
            }
            _ => self.list_lambda_method(list, item, name, arguments, ty, (&temp, &index)),
        }

        temp
    }

    /// `list.add(item)`: a list with room for the strings of both.
    fn add(&mut self, list: &str, item: &Type, arguments: &[Argument], ty: &Type) -> String {
        let Some(argument) = arguments.first() else {
            return "0".to_string();
        };

        let (added, added_type) = self.eval(&argument.value, Some(item));
        let wider = self.gen.wider(item, &added_type);
        let ty = self.gen.keep_bound(&Type::list(wider), ty);

        let Type::List(kept) = &ty else {
            return "0".to_string();
        };

        // The added item as one of the list's type, but for its bounds.
        let kept = (**kept).clone();
        let given = self.gen.keep_bound(&added_type, &kept);
        let added = self.convert(added, &added_type, &given);

        let capacity = self.gen.sizes.list_capacity(item);
        let temp = self.temp(&ty);
        self.copy_into(&temp, list, &Type::list(item.clone()), &ty);

        self.line(format!(
            "CROFT_CHECK({temp}.length < {capacity}, \"a list is full\");"
        ));

        self.open(format!("if ({temp}.length < {capacity})"));
        let slot = format!("{temp}.items[{temp}.length]");
        self.copy_into(&slot, &added, &given, &kept);
        self.line(format!("{temp}.length++;"));
        self.close();

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
    pub fn list_literal(&mut self, items: &[croft_syntax::ast::Expr], ty: &Type) -> String {
        if *ty == Type::Bytes {
            return self.bytes_literal(items);
        }

        let Type::List(item) = ty else {
            return "0".to_string();
        };

        // An empty list holds no strings.
        let ty = if items.is_empty() {
            self.gen.narrowest(ty)
        } else {
            ty.clone()
        };

        let list = self.temp(&ty);
        let capacity = self.gen.sizes.list_capacity(item);

        self.line(format!(
            "{list}.length = {};",
            items.len().min(capacity as usize)
        ));

        if items.len() as i64 > capacity {
            self.line(format!(
                "CROFT_CHECK(0, \"a list has more than {capacity} items\");"
            ));
        }

        for (index, value) in items.iter().take(capacity as usize).enumerate() {
            self.assign(&format!("{list}.items[{index}]"), value, item);
        }

        list
    }
}
