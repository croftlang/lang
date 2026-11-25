//! Names in a function: C variables, temporaries, and what each Croft name refers to.

use std::collections::HashMap;

use croft_analysis::Type;

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    /// Declares a C variable with a name not used yet in this function.
    pub fn variable(&mut self, base: &str, ty: &Type) -> String {
        let mut name = base.to_string();
        let mut count = 1;

        while !self.names.insert(name.clone()) {
            count += 1;
            name = format!("{base}_{count}");
        }

        let c_type = self.gen.c_type(ty);
        self.declarations.push(format!("    {c_type} {name};"));

        name
    }

    /// Declares a C variable of a type that is not a Croft type, such as `croft_event`.
    pub fn raw_variable(&mut self, base: &str, c_type: &str) -> String {
        let mut name = base.to_string();
        let mut count = 1;

        while !self.names.insert(name.clone()) {
            count += 1;
            name = format!("{base}_{count}");
        }

        self.declarations.push(format!("    {c_type} {name};"));

        name
    }

    /// A temporary. Options and results start cleared, so a `None` or a
    /// failure never carries an unset value around.
    pub fn temp(&mut self, ty: &Type) -> String {
        let temp = self.variable("t", ty);

        if matches!(ty, Type::Option(_) | Type::Result(..)) {
            self.line(format!("croft_clear(&{temp}, sizeof {temp});"));
        }

        temp
    }

    /// Makes a Croft name refer to a C expression in the innermost scope.
    pub fn bind(&mut self, name: &str, c: String, ty: Type) {
        self.scopes
            .last_mut()
            .expect("a scope")
            .insert(name.to_string(), (c, ty));
    }

    pub fn lookup(&self, name: &str) -> Option<(String, Type)> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    /// Reserves a C name, such as a parameter's, so no variable reuses it.
    pub fn reserve(&mut self, name: &str) {
        self.names.insert(name.to_string());
    }

    /// Records a C parameter, so an unused one can be marked as such.
    pub fn parameter(&mut self, name: &str) {
        self.reserve(name);
        self.parameters.push(name.to_string());
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
