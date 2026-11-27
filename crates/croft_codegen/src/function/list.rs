//! The built-in methods of lists, written as loops. A list of entities is
//! also looked up, replaced and removed by identity.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Argument, BinaryOp, Expr, ExprKind};

use super::Writer;
use crate::types::bounds::{is_bytes, list_item, unbounded};

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
                "remove" | "replace" | "drop" | "take" | "filter" | "reverse" | "sort" | "sort_by"
                | "unique",
                _,
            ) => self.gen.keep_bound(list_type, ty),
            ("map", Some(mapped)) => self.gen.with_item(list_type, mapped.clone()),
            _ => ty.clone(),
        };

        let ty = &ty;

        if name == "add" {
            return self.add(list, list_type, arguments, ty);
        }

        if name == "insert" {
            return self.insert(list, list_type, arguments, ty);
        }

        if name == "fold" {
            return self.fold(list, item, arguments, ty);
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
            "unique" => {
                let seen = self.index_variable();
                let earlier = format!("{temp}.items[{seen}]");

                let same = match &identity {
                    Some((field, _)) => format!("({earlier}.{field} == {current}.{field})"),
                    None => self.equal(&earlier, &current, item),
                };

                self.line(format!("{temp}.length = 0;"));
                self.open(&each);

                self.open(format!(
                    "for ({seen} = 0; {seen} < {temp}.length; {seen}++)"
                ));

                self.open(format!("if ({same})"));
                self.line("break;");
                self.close();
                self.close();

                self.line(format!(
                    "if ({seen} == {temp}.length) {temp}.items[{temp}.length++] = {current};"
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

    /// `list.insert(place, item)`: a list one item longer, with `item` at
    /// `place` and the items from there on one place later. A place below
    /// zero or past the end stops a checked build; otherwise the item goes
    /// at the start or the end.
    fn insert(
        &mut self,
        list: &str,
        list_type: &Type,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let (Some(place), Some(argument), Some(item)) =
            (arguments.first(), arguments.get(1), list_item(list_type))
        else {
            return "0".to_string();
        };

        // Read as a `croft_int`, so a place kept in an unsigned byte can be
        // compared with zero without gcc warning that it is never below it.
        let wanted = self.value(&place.value, &Type::Int);
        let at = self.raw_variable("at", "croft_int");
        self.line(format!("{at} = {wanted};"));

        let (added, added_type) = self.eval(&argument.value, Some(item));
        let wider = self.gen.wider(item, &added_type);
        let longer = self.gen.list_type(wider, self.gen.capacity(list_type) + 1);
        let ty = self.gen.keep_bound(&longer, ty);

        let Some(kept) = list_item(&ty) else {
            return "0".to_string();
        };

        let kept = kept.clone();
        let given = self.gen.keep_bound(&added_type, &kept);
        let added = self.convert(added, &added_type, &given);
        let index = self.index_variable();

        let capacity = self.gen.capacity(&ty);
        let temp = self.temp(&ty);
        self.copy_into(&temp, list, list_type, &ty);

        self.line(format!(
            "CROFT_CHECK({at} >= 0 && {at} <= (croft_int) {temp}.length, \"a list has no place to insert at\");"
        ));

        self.line(format!(
            "CROFT_CHECK({temp}.length < {capacity}, \"a list is full\");"
        ));

        self.line(format!("if ({at} < 0) {at} = 0;"));

        self.line(format!(
            "if ({at} > (croft_int) {temp}.length) {at} = (croft_int) {temp}.length;"
        ));

        self.open(format!("if ({temp}.length < {capacity})"));

        self.open(format!(
            "for ({index} = {temp}.length; {index} > (croft_u16) {at}; {index}--)"
        ));

        self.line(format!(
            "{temp}.items[{index}] = {temp}.items[{index} - 1];"
        ));

        self.close();

        let slot = format!("{temp}.items[{at}]");
        self.copy_into(&slot, &added, &given, &kept);
        self.line(format!("{temp}.length++;"));
        self.close();

        temp
    }

    /// `list.fold(start, (total, item) -> ...)`: `total` starts as `start`,
    /// and the lambda gives the next one for each item in turn. It is kept
    /// in the full size of its type, since it can grow with every item.
    fn fold(&mut self, list: &str, item: &Type, arguments: &[Argument], ty: &Type) -> String {
        let (Some(start), Some(lambda)) = (arguments.first(), arguments.get(1)) else {
            return "0".to_string();
        };

        let ExprKind::Lambda(parameters, body) = &lambda.value.kind else {
            return "0".to_string();
        };

        let ty = self.gen.owned_type(&unbounded(ty));
        let total = self.temp(&ty);
        let index = self.index_variable();

        self.assign(&total, &start.value, &ty);

        self.open(format!(
            "for ({index} = 0; {index} < {list}.length; {index}++)"
        ));

        self.push_scope();

        if let [total_name, item_name] = parameters.as_slice() {
            self.bind(&total_name.name, total.clone(), ty.clone());
            self.bind(
                &item_name.name,
                format!("{list}.items[{index}]"),
                item.clone(),
            );
        }

        self.assign(&total, body, &ty);
        self.pop_scope();
        self.close();

        total
    }

    /// `set items = items.add(item)` or `set items = items + more`, where
    /// `target` is `items`, a list of type `ty` named `name`: the new items
    /// go straight onto its end, with no copy of the list. Gives back
    /// whether `value` was one of these and so has been written.
    pub fn append_in_place(&mut self, target: &str, ty: &Type, name: &str, value: &Expr) -> bool {
        let Some(item) = list_item(ty).cloned() else {
            return false;
        };

        // The list being set, read by its name.
        let is_target = |writer: &Self, expr: &Expr| {
            matches!(&expr.kind, ExprKind::Name(read) if read == name)
                && matches!(writer.type_of(expr, None), Type::List(_))
        };

        match &value.kind {
            ExprKind::Call(callee, arguments) => {
                let ExprKind::Field(receiver, method) = &callee.kind else {
                    return false;
                };

                let [argument] = arguments.as_slice() else {
                    return false;
                };

                let is_method = matches!(self.gen.facts.meaning(value.id), Some(Meaning::Method));

                if !is_method || method.name != "add" || !is_target(self, receiver) {
                    return false;
                }

                self.push_in_place(target, ty, &item, &argument.value);
                true
            }
            ExprKind::Binary(BinaryOp::Add, left, right) if is_target(self, left) => {
                match &right.kind {
                    ExprKind::List(items) => {
                        for added in items {
                            self.push_in_place(target, ty, &item, added);
                        }
                    }
                    _ => self.push_all_in_place(target, ty, &item, right),
                }

                true
            }
            _ => false,
        }
    }

    /// Adds `added` to the end of `target`, a list of type `ty` holding
    /// items of `item`, stopping a checked build when it is full.
    fn push_in_place(&mut self, target: &str, ty: &Type, item: &Type, added: &Expr) {
        let (added, added_type) = self.eval(added, Some(item));
        let given = self.gen.keep_bound(&added_type, item);
        let added = self.convert(added, &added_type, &given);
        let capacity = self.gen.capacity(ty);

        self.line(format!(
            "CROFT_CHECK({target}.length < {capacity}, \"a list is full\");"
        ));

        self.open(format!("if ({target}.length < {capacity})"));
        let slot = format!("{target}.items[{target}.length]");
        self.copy_into(&slot, &added, &given, item);
        self.line(format!("{target}.length++;"));
        self.close();
    }

    /// Adds every item of the list `more` to the end of `target`, a list of
    /// type `ty` holding items of `item`. How many there are is read first,
    /// so `items + items` adds each once.
    fn push_all_in_place(&mut self, target: &str, ty: &Type, item: &Type, more: &Expr) {
        let (more, more_type) = self.eval(more, Some(ty));

        let Some(more_item) = list_item(&more_type).cloned() else {
            return;
        };

        let capacity = self.gen.capacity(ty);
        let count = self.raw_variable("count", "croft_u16");
        let index = self.index_variable();

        self.line(format!("{count} = {more}.length;"));
        self.open(format!("for ({index} = 0; {index} < {count}; {index}++)"));

        self.line(format!(
            "CROFT_CHECK({target}.length < {capacity}, \"a list is full\");"
        ));

        self.open(format!("if ({target}.length < {capacity})"));
        let slot = format!("{target}.items[{target}.length]");
        let given = format!("{more}.items[{index}]");
        self.copy_into(&slot, &given, &more_item, item);
        self.line(format!("{target}.length++;"));
        self.close();
        self.close();
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
