//! Names in a function: C variables, temporaries, and what each Croft name refers to.

use std::collections::HashMap;

use croft_analysis::Type;

use super::{Into, Returns, Writer};
use crate::types::bounds::has_bound;

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

        self.typed(name, ty)
    }

    /// Records that `c`, a C name or place, holds a value of `ty`, when that
    /// has a bounded string in it, and gives back `c`.
    pub fn typed(&mut self, c: String, ty: &Type) -> String {
        if has_bound(ty) {
            self.bounded.insert(c.clone(), ty.clone());
        }

        c
    }

    /// Notes a value of `ty` written to `target`, when that is the result.
    pub fn note_result(&mut self, target: &str, ty: &Type) {
        let Returns::Value(result) = &self.returns else {
            return;
        };

        if target != "(*out)" {
            return;
        }

        // What leaves the function is copied out of anything it borrows from.
        let given = self.gen.owned_type(&self.gen.keep_bound(ty, result));

        self.returned = Some(match &self.returned {
            Some(earlier) => self.gen.wider(earlier, &given),
            None => given,
        });
    }

    /// Whether the string a view borrows stays put until the function
    /// returns: a literal, a parameter, a part of one, or a temporary.
    pub fn is_stable(&self, view: &str) -> bool {
        if view.starts_with("croft_string_view_from(\"") || self.views.contains(view) {
            return true;
        }

        if matches!(self.bounded.get(view), Some(Type::Borrowed(_))) {
            return true;
        }

        let Some(inner) = view
            .strip_prefix("CROFT_VIEW(")
            .and_then(|v| v.strip_suffix(')'))
        else {
            return false;
        };

        let root = inner.split(['.', '[']).next().unwrap_or(inner);

        self.stable.contains(root) || matches!(self.bounded.get(inner), Some(Type::Borrowed(_)))
    }

    /// The type of the value `c` holds: `ty` with the bounds of its strings,
    /// when `c` was recorded with them.
    pub fn bounded_type(&self, c: &str, ty: &Type) -> Type {
        match self.bounded.get(c) {
            Some(bounded) => self.gen.keep_bound(bounded, ty),
            None => ty.clone(),
        }
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

    /// A new C array of `length` items of `c_type`, whose name starts with
    /// `base`. `length` is a whole number, so `croft size` can count the
    /// array in the function's stack frame.
    pub fn array_variable(&mut self, base: &str, c_type: &str, length: &str) -> String {
        let mut name = base.to_string();
        let mut count = 1;

        while !self.names.insert(name.clone()) {
            count += 1;
            name = format!("{base}_{count}");
        }

        self.declarations
            .push(format!("    {c_type} {name}[{length}];"));

        name
    }

    /// A temporary. Options and results start cleared, so a `None` or a
    /// failure never carries an unset value around.
    pub fn temp(&mut self, ty: &Type) -> String {
        let c_type = self.gen.c_type(ty);

        let temp = match self.free.iter().position(|(free, _)| *free == c_type) {
            Some(index) => {
                let (_, name) = self.free.remove(index);
                self.bounded.remove(&name);
                self.typed(name, ty)
            }
            None => self.variable("t", ty),
        };

        self.stable.insert(temp.clone());
        self.handed.push((c_type, temp.clone()));

        if matches!(ty, Type::Option(_) | Type::Result(..)) {
            self.line(format!("croft_clear(&{temp}, sizeof {temp});"));
        }

        temp
    }

    /// Where a value of `ty` is made: straight in `into` when that place has
    /// its C type, or else in a new temporary. Either starts cleared when it
    /// is an option or a result.
    pub fn place_for(&mut self, ty: &Type, into: Into<'_>) -> String {
        match into {
            Some((target, wanted)) if self.gen.c_type(wanted) == self.gen.c_type(ty) => {
                if matches!(ty, Type::Option(_) | Type::Result(..)) {
                    self.line(format!("croft_clear(&{target}, sizeof {target});"));
                }

                self.note_result(target, ty);
                target.to_string()
            }
            _ => self.temp(ty),
        }
    }

    /// Marks where the temporaries handed out from now on begin.
    pub fn temps_mark(&self) -> usize {
        self.handed.len()
    }

    /// Whether `c` is a temporary handed out since `mark`.
    pub fn handed_since(&self, mark: usize, c: &str) -> bool {
        self.handed[mark..].iter().any(|(_, name)| name == c)
    }

    /// Lets the temporaries handed out since `mark` be handed out again: the
    /// code that used them, a branch that has returned or a `match` arm that
    /// has written its value, is done with them.
    pub fn release_since(&mut self, mark: usize) {
        for (c_type, name) in &self.handed[mark..] {
            if !self.free.iter().any(|(_, free)| free == name) {
                self.free.push((c_type.clone(), name.clone()));
            }
        }
    }

    /// Makes a Croft name refer to a C expression in the innermost scope.
    pub fn bind(&mut self, name: &str, c: String, ty: Type) {
        let c = self.typed(c, &ty);

        self.scopes
            .last_mut()
            .expect("a scope")
            .insert(name.to_string(), (c, ty));
    }

    /// What a Croft name refers to, from the innermost scope out.
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

    /// Starts a new innermost scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Ends the innermost scope, and the names it held.
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
