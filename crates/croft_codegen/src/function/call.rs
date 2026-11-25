//! Calls to functions, translations, constraints and services declared in
//! Croft, to enum variants, and to native C code.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Argument, Expr, ExprKind};

use super::{Into, Writer};
use crate::names::c_name;
use crate::types::bounds::unbounded;

impl<'g, 'a> Writer<'g, 'a> {
    /// A call, giving back its value in `into` when it can be written there.
    pub fn call(
        &mut self,
        value: &Expr,
        callee: &Expr,
        arguments: &[Argument],
        ty: &Type,
        into: Into<'_>,
    ) -> String {
        match self.gen.facts.meaning(value.id).cloned() {
            Some(Meaning::Method) => {
                let ExprKind::Field(receiver, method) = &callee.kind else {
                    return "0".to_string();
                };

                self.method(receiver, &method.name, arguments, ty, into)
            }
            Some(Meaning::Builtin(name)) => self.builtin(&name, arguments, ty, into),
            Some(Meaning::Variant(owner, variant, error)) => {
                self.variant_call(&owner, &variant, error.as_ref(), arguments, ty, into)
            }
            Some(Meaning::Declared(found)) => self.call_declared(&found, arguments, ty, into),
            _ => "0".to_string(),
        }
    }

    fn call_declared(
        &mut self,
        found: &Resolved,
        arguments: &[Argument],
        ty: &Type,
        into: Into<'_>,
    ) -> String {
        if found.kind == SymbolKind::Value {
            let inner = self
                .gen
                .model
                .single_value_inner(ty)
                .unwrap_or(Type::Unknown);

            let value = arguments.first().map(|argument| &argument.value);
            let value = value.map_or("0".to_string(), |value| self.value(value, &inner));
            return self.wrap_single_value(value, ty);
        }

        let parameters = self.gen.parameters_of(found);
        let passed = self.pass_all(arguments, &parameters);

        if found.kind != SymbolKind::Service {
            let result = match self.gen.result_of(found) {
                Some(result) => self.gen.keep_bound(&result, ty),
                None => ty.clone(),
            };

            return self.call_to(&c_name(found), passed, &result, into);
        }

        // The events the service raises, and kept, are handled once it ends.
        let mark = self.raw_variable("mark", "croft_u16");
        self.line(format!("{mark} = croft_events_mark();"));

        let result = self.call_to(&c_name(found), passed, ty, into);

        self.line(format!("croft_events_dispatch_from({mark});"));

        result
    }

    /// `native zx_cls()`: arguments as Croft passes them, and a pointer for the
    /// result when there is one. The prototype is written from the call.
    pub fn native(
        &mut self,
        function: &str,
        arguments: &[Argument],
        want: Option<&Type>,
    ) -> String {
        let mut passed = Vec::new();
        let mut parameters = Vec::new();

        for argument in arguments {
            let ty = self.type_of(&argument.value, None);

            if ty == Type::String {
                parameters.push("croft_string_view".to_string());
                passed.push(self.view(&argument.value));
                continue;
            }

            let c = self.value(&argument.value, &ty);
            let c_type = self.gen.c_type(&ty);

            parameters.push(if self.gen.is_scalar(&ty) {
                c_type
            } else {
                format!("const {c_type} *")
            });

            passed.push(self.pass(c, &ty));
        }

        // Native C always writes a full string, whatever size is wanted.
        let result = want.map_or(Type::Unit, unbounded);
        parameters.push(format!("{} *", self.gen.c_type(&result)));

        self.gen.natives.borrow_mut().push((
            function.to_string(),
            format!("void {function}({});", parameters.join(", ")),
        ));

        self.call_into(function, passed, &result)
    }
}
