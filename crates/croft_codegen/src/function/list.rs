//! The built-in methods of lists, written as loops. A list of entities is
//! also looked up, replaced and removed by identity.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::Type;
use croft_syntax::ast::Argument;

use super::Writer;
use crate::types::bounds::{is_bytes, list_item};

impl<'g, 'a> Writer<'g, 'a> {
    /// `list.name(arguments)`, on `list`, a list of type `list_type`.
    pub fn list_method(
        &mut self,
        list: &str,
        list_type: &Type,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let Some(item) = list_item(list_type) else {
            return "0".to_string();
        };

        let item = &item.clone();
        let identity = self.identity_field(item);

        let key_type = match &identity {
            Some((_, key)) => key.clone(),
            None => Type::Int,
        };

        let first = |writer: &mut Self, want: &Type| match arguments.first() {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };

        // The methods that give back the list's own items keep their bounds,
        // and those that give back a list are no longer than it.
        let ty = match (name, list_item(ty)) {
            ("get" | "find" | "min_by" | "max_by", _) => {
                self.gen.keep_bound(&Type::option(item.clone()), ty)
            }
            (
                "remove" | "replace" | "drop" | "take" | "filter" | "reverse" | "sort" | "sort_by",
                _,
            ) => self.gen.keep_bound(list_type, ty),
            ("map", Some(mapped)) => self.gen.with_item(list_type, mapped.clone()),
            _ => ty.clone(),
        };

        let ty = &ty;

        if name == "add" {
            return self.add(list, list_type, arguments, ty);
        }

        if name == "join" {
            return self.join(list, list_type, arguments);
        }

        if name == "sort" || name == "sort_by" {
            return self.sort(list, item, arguments, ty);
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

                // An index known not to be negative, such as a counter kept
                // in an unsigned byte, needs no test that it is: gcc warns
                // that one is always true.
                let never_negative = (arguments.first())
                    .and_then(|argument| self.range_of(&argument.value))
                    .is_some_and(|range| range.low >= 0);

                let test = if never_negative {
                    format!("{position} < (croft_int) {list}.length")
                } else {
                    format!("{position} >= 0 && {position} < (croft_int) {list}.length")
                };

                self.line(format!("{temp}.some = {test};"));

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
            "index_of" => {
                let wanted = first(self, item);
                let test = matches(self, &wanted);

                self.line(format!("{temp}.some = 0;"));
                self.line(format!("{temp}.value = 0;"));
                self.open(&each);

                self.line(format!(
                    "if (!{temp}.some && {test}) {{ {temp}.some = 1; {temp}.value = {index}; }}"
                ));

                self.close();
            }
            "reverse" => {
                self.line(format!("{temp}.length = {list}.length;"));
                self.open(&each);

                self.line(format!(
                    "{temp}.items[{list}.length - 1 - {index}] = {current};"
                ));

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

    /// `list.add(item)`: a list one item longer, with room for the strings
    /// of both.
    fn add(&mut self, list: &str, list_type: &Type, arguments: &[Argument], ty: &Type) -> String {
        let (Some(argument), Some(item)) = (arguments.first(), list_item(list_type)) else {
            return "0".to_string();
        };

        let (added, added_type) = self.eval(&argument.value, Some(item));
        let wider = self.gen.wider(item, &added_type);
        let longer = self.gen.list_type(wider, self.gen.capacity(list_type) + 1);
        let ty = self.gen.keep_bound(&longer, ty);

        let Some(kept) = list_item(&ty) else {
            return "0".to_string();
        };

        // The added item as one of the list's type, but for its bounds.
        let kept = kept.clone();
        let given = self.gen.keep_bound(&added_type, &kept);
        let added = self.convert(added, &added_type, &given);

        let capacity = self.gen.capacity(&ty);
        let temp = self.temp(&ty);
        self.copy_into(&temp, list, list_type, &ty);

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

    /// `names.join(", ")`: each string, with the separator between each two.
    fn join(&mut self, list: &str, list_type: &Type, arguments: &[Argument]) -> String {
        let item = list_item(list_type).cloned().unwrap_or(Type::String);
        let item = &item;

        let (value, separator_type) = match arguments.first() {
            Some(argument) => self.bounded_view(&argument.value),
            None => ("croft_string_view_from(\"\", 0)".to_string(), Type::Text(1)),
        };

        let separator = self.raw_variable("separator", "croft_string_view");
        self.line(format!("{separator} = {value};"));

        let capacity = self.gen.capacity(list_type);
        let most =
            capacity * self.gen.bound(item) + (capacity - 1) * self.gen.bound(&separator_type);
        let joined = self.temp(&self.gen.text_type(most));
        let index = self.index_variable();

        self.line(format!("{joined}.length = 0;"));
        self.open(format!(
            "for ({index} = 0; {index} < {list}.length; {index}++)"
        ));

        self.line(format!(
            "if ({index} > 0) croft_string_join(CROFT_VIEW({joined}), {separator}, CROFT_OUT({joined}));"
        ));

        self.line(format!(
            "croft_string_join(CROFT_VIEW({joined}), CROFT_VIEW({list}.items[{index}]), CROFT_OUT({joined}));"
        ));

        self.close();

        joined
    }

    /// `sort` and `sort_by`: a copy in order, numbers by size and strings
    /// byte by byte, or by what the lambda gives each item. Items that are
    /// the same keep their order. A merge sort puts the items' places in
    /// order, so each item is copied once, however long it is; `sort_by`
    /// works out each item's key once, first.
    fn sort(&mut self, list: &str, item: &Type, arguments: &[Argument], ty: &Type) -> String {
        let sorted = self.temp(ty);
        let index = self.index_variable();
        // Written as a number, the list's own capacity, so `croft size` can
        // count the arrays in the function's stack frame.
        let capacity = self.gen.capacity(ty).to_string();
        let order = self.array_variable("order", "croft_u16", &capacity);
        let room = self.array_variable("room", "croft_u16", &capacity);

        let each = format!("for ({index} = 0; {index} < {list}.length; {index}++)");

        let keys = if arguments.is_empty() {
            format!("({} *) {list}.items", self.gen.c_type(item))
        } else {
            let keys = self.array_variable("keys", "croft_int", &capacity);
            let current = format!("{list}.items[{index}]");

            self.open(&each);
            let value = self.lambda(arguments, current, item, &Type::Int);
            self.line(format!("{keys}[{index}] = {value};"));
            self.close();

            keys
        };

        let order_of = if arguments.is_empty() {
            self.gen.sort_order_for(item)
        } else {
            self.gen.sort_order_for(&Type::Int)
        };

        self.line(format!(
            "{order_of}({keys}, {list}.length, {order}, {room});"
        ));
        self.line(format!("{sorted}.length = {list}.length;"));
        self.open(&each);
        self.line(format!(
            "{sorted}.items[{index}] = {list}.items[{order}[{index}]];"
        ));
        self.close();

        sorted
    }

    /// The identity field of an entity, and its type.
    fn identity_field(&self, item: &Type) -> Option<(String, Type)> {
        let entity = item.named(&[SymbolKind::Entity])?;
        let fields = self.gen.model.fields(entity);
        let (name, ty, _) = fields.into_iter().find(|(_, _, field)| field.is_identity)?;
        Some((name, ty))
    }

    /// `[a, b, c]`: a list of exactly these items, with room for no more.
    pub fn list_literal(&mut self, items: &[croft_syntax::ast::Expr], ty: &Type) -> String {
        if is_bytes(ty) {
            return self.bytes_literal(items);
        }

        let Some(item) = list_item(ty) else {
            return "0".to_string();
        };

        // An empty list holds no strings.
        let ty = if items.is_empty() {
            self.gen.narrowest(ty)
        } else {
            self.gen.list_type(item.clone(), items.len() as i64)
        };

        let list = self.temp(&ty);
        self.fill_list(&list, items, &ty);

        list
    }

    /// Writes the items of `[a, b, c]` straight into `list`, a list of type
    /// `ty`, stopping a checked build when there are more than it holds.
    pub fn fill_list(&mut self, list: &str, items: &[croft_syntax::ast::Expr], ty: &Type) {
        let Some(item) = list_item(ty) else {
            return;
        };

        let item = &item.clone();
        let capacity = self.gen.capacity(ty);

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
    }
}
