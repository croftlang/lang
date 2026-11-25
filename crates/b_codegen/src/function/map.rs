//! Maps in C: a length, and keys and values side by side, as many as the
//! longest `Length` constraint on that type of map allows. Lookups go
//! through the keys in order, which is quick for the small maps a domain
//! keeps and needs no hashing on an 8-bit machine.

use b_analysis::Type;
use b_syntax::ast::{Argument, Expr};

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn map_literal(&mut self, entries: &[(Expr, Expr)], ty: &Type) -> String {
        let Type::Map(key, value) = ty else {
            return "0".to_string();
        };
        let map = self.temp(ty);
        self.line(format!("{map}.length = 0;"));
        for (entry_key, entry_value) in entries {
            let k = self.value(entry_key, key);
            let v = self.value(entry_value, value);
            self.map_set_in_place(&map, &k, &v, ty);
        }
        map
    }

    pub fn map_method(
        &mut self,
        map: &str,
        key: &Type,
        value: &Type,
        name: &str,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let map_type = Type::map(key.clone(), value.clone());
        let argument = |writer: &mut Self, index: usize, want: &Type| match arguments.get(index) {
            Some(argument) => writer.value(&argument.value, want),
            None => "0".to_string(),
        };
        let k = argument(self, 0, key);
        match name {
            "get" | "contains" => {
                let index = self.map_find(map, &k, key);
                if name == "contains" {
                    return format!("({index} < {map}.length)");
                }
                let found = self.temp(ty);
                self.open(format!("if ({index} < {map}.length)"));
                self.line(format!("{found}.some = 1;"));
                self.line(format!("{found}.value = {map}.values[{index}];"));
                self.close();
                found
            }
            "set" => {
                let v = argument(self, 1, value);
                let copy = self.temp(&map_type);
                self.line(format!("{copy} = {map};"));
                self.map_set_in_place(&copy, &k, &v, &map_type);
                copy
            }
            "remove" => {
                let copy = self.temp(&map_type);
                self.line(format!("{copy} = {map};"));
                let index = self.map_find(&copy, &k, key);
                self.open(format!("if ({index} < {copy}.length)"));
                self.line(format!("{copy}.length--;"));
                self.line(format!(
                    "{copy}.keys[{index}] = {copy}.keys[{copy}.length];"
                ));
                self.line(format!(
                    "{copy}.values[{index}] = {copy}.values[{copy}.length];"
                ));
                self.close();
                copy
            }
            _ => "0".to_string(),
        }
    }

    /// `map.keys` or `map.values`, as a list.
    pub fn map_list(&mut self, map: &str, item: &Type, field: &str) -> String {
        let list = self.temp(&Type::list(item.clone()));
        let index = self.variable("index", &Type::Int);
        self.line(format!("{list}.length = {map}.length;"));
        self.open(format!(
            "for ({index} = 0; {index} < {map}.length; {index}++)"
        ));
        self.line(format!("{list}.items[{index}] = {map}.{field}[{index}];"));
        self.close();
        list
    }

    /// Where `key` is in the map, or its length when it isn't there.
    fn map_find(&mut self, map: &str, key: &str, key_type: &Type) -> String {
        let index = self.variable("index", &Type::Int);
        let same = self.equal(&format!("{map}.keys[{index}]"), key, key_type);
        self.open(format!(
            "for ({index} = 0; {index} < {map}.length; {index}++)"
        ));
        self.open(format!("if ({same})"));
        self.line("break;");
        self.close();
        self.close();
        index
    }

    /// Sets a key's value in `map`, adding the key when it isn't there.
    fn map_set_in_place(&mut self, map: &str, key: &str, value: &str, ty: &Type) {
        let Type::Map(key_type, _) = ty else {
            return;
        };
        let capacity = self.gen.sizes.map_capacity(ty);
        let index = self.map_find(map, key, key_type);
        self.open(format!(
            "if ({index} == {map}.length && {map}.length < {capacity})"
        ));
        self.line(format!("{map}.keys[{index}] = {key};"));
        self.line(format!("{map}.length++;"));
        self.close();
        self.line(format!(
            "B_CHECK({index} < {map}.length, \"a map is full\");"
        ));
        self.open(format!("if ({index} < {map}.length)"));
        self.line(format!("{map}.values[{index}] = {value};"));
        self.close();
    }
}
