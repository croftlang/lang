//! Sets in C: a length and the items, as many as the longest `Length`
//! constraint on that type of set allows, kept in the order they were added.
//! Like a map's keys, finding an item goes through them in order, which is
//! quick for the small sets a domain keeps and needs no hashing on an 8-bit
//! machine.

use croft_analysis::Type;
use croft_syntax::ast::{Argument, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// `[1, 2, 3]` where a set is wanted: a set holding these items, each once.
    pub fn set_literal(&mut self, items: &[Expr], ty: &Type) -> String {
        let Type::Set(item) = ty else {
            return "0".to_string();
        };

        let set = self.temp(ty);
        self.line(format!("{set}.length = 0;"));

        for value in items {
            let added = self.value(value, item);
            self.set_add_in_place(&set, &added, ty);
        }

        set
    }

    /// A method of `set`, a set of `item`.
    pub fn set_method(
        &mut self,
        set: &str,
        item: &Type,
        name: &str,
        arguments: &[Argument],
    ) -> String {
        let set_type = Type::set(item.clone());

        let given = match arguments.first() {
            Some(argument) => self.value(&argument.value, item),
            None => "0".to_string(),
        };

        match name {
            "contains" => {
                let index = self.set_find(set, &given, item);
                format!("({index} < {set}.length)")
            }
            "add" => {
                let copy = self.temp(&set_type);
                self.line(format!("{copy} = {set};"));
                self.set_add_in_place(&copy, &given, &set_type);
                copy
            }
            "remove" => {
                let copy = self.temp(&set_type);
                self.line(format!("{copy} = {set};"));

                // The items after it move down one place, so the rest keep
                // their order.
                let index = self.set_find(&copy, &given, item);
                self.open(format!("if ({index} < {copy}.length)"));
                self.line(format!("{copy}.length--;"));

                self.open(format!("for (; {index} < {copy}.length; {index}++)"));

                self.line(format!(
                    "{copy}.items[{index}] = {copy}.items[{index} + 1];"
                ));

                self.close();
                self.close();

                copy
            }
            _ => "0".to_string(),
        }
    }

    /// `set.items`: the set's items as a list, in the order they were added.
    pub fn set_items(&mut self, set: &str, set_type: &Type, item: &Type) -> String {
        let items = self.gen.sizes.set_capacity(set_type);
        let list = self.temp(&self.gen.list_type(item.clone(), items));
        let index = self.index_variable();

        self.line(format!("{list}.length = {set}.length;"));

        self.open(format!(
            "for ({index} = 0; {index} < {set}.length; {index}++)"
        ));

        self.line(format!("{list}.items[{index}] = {set}.items[{index}];"));
        self.close();

        list
    }

    /// Where `item` is in the set, or its length when it isn't there.
    fn set_find(&mut self, set: &str, item: &str, item_type: &Type) -> String {
        let index = self.index_variable();
        let same = self.equal(&format!("{set}.items[{index}]"), item, item_type);

        self.open(format!(
            "for ({index} = 0; {index} < {set}.length; {index}++)"
        ));

        self.open(format!("if ({same})"));
        self.line("break;");
        self.close();
        self.close();

        index
    }

    /// Adds `item` to the end of `set` when it isn't there already. A checked
    /// build stops when the set is full.
    fn set_add_in_place(&mut self, set: &str, item: &str, ty: &Type) {
        let Type::Set(item_type) = ty else {
            return;
        };

        let capacity = self.gen.sizes.set_capacity(ty);
        let index = self.set_find(set, item, item_type);

        self.line(format!(
            "CROFT_CHECK({index} < {set}.length || {set}.length < {capacity}, \"a set is full\");"
        ));

        self.open(format!(
            "if ({index} == {set}.length && {set}.length < {capacity})"
        ));

        self.line(format!("{set}.items[{index}] = {item};"));
        self.line(format!("{set}.length++;"));
        self.close();
    }
}
