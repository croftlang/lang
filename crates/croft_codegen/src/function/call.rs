//! Calls to functions, translations, constraints and services declared in
//! Croft, to enum variants, and to native C code.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Argument, Expr, ExprKind};

use super::{Into, Writer};
use crate::names::c_name;
use crate::types::bounds::{is_bytes, unbounded};

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

    /// A call to a function, translation, constraint, service or
    /// single-value type declared in Croft.
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

        // The events the service raises and keeps are handed to policies
        // once it ends.
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
        self.native_into(function, arguments, want, None)
    }

    /// A native call, as `native` writes it, giving back `Bytes` in `into`
    /// when that place has their C type.
    pub fn native_into(
        &mut self,
        function: &str,
        arguments: &[Argument],
        want: Option<&Type>,
        into: Into<'_>,
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

            if is_bytes(&ty) {
                parameters.push("croft_bytes_view".to_string());
                passed.push(self.bytes_view(&argument.value));
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

        if let Some(wanted) = want.filter(|want| gives_bytes(want)) {
            return self.native_bytes(function, (passed, parameters), wanted, into);
        }

        // Native C always writes a full string or list, whatever size is
        // wanted, but `Bytes` in the size wanted, as `Storage.load` has room
        // for the largest save.
        let result = want.map_or(Type::Unit, full_keeping_bytes);
        parameters.push(format!("{} *", self.gen.c_type(&result)));

        self.gen.natives.borrow_mut().push((
            function.to_string(),
            format!("void {function}({});", parameters.join(", ")),
        ));

        self.call_into(function, passed, &result)
    }

    /// A native giving back `Bytes`, alone or in a `Result`: it is given a
    /// `croft_bytes_writer` for them, so they go straight into `wanted`'s
    /// size, however large, and then the `Result` without them, if any.
    fn native_bytes(
        &mut self,
        function: &str,
        (mut passed, mut parameters): (Vec<String>, Vec<String>),
        wanted: &Type,
        into: Into<'_>,
    ) -> String {
        let wanted = self.gen.owned_type(wanted);
        let temp = self.place_for(&wanted, into);
        let writer = self.raw_variable("writer", "croft_bytes_writer");

        let bytes = match &wanted {
            Type::Result(..) => format!("{temp}.value"),
            _ => temp.clone(),
        };

        self.line(format!(
            "{writer} = croft_bytes_writer_to(CROFT_BYTES_OUT({bytes}));"
        ));

        parameters.push("croft_bytes_writer *".to_string());
        passed.push(format!("&{writer}"));

        let outcome = match &wanted {
            Type::Result(_, error) => Some(Type::result(Type::Unit, (**error).clone())),
            _ => None,
        };

        let done = outcome.as_ref().map(|outcome| {
            let done = self.temp(outcome);
            parameters.push(format!("{} *", self.gen.c_type(outcome)));
            passed.push(format!("&{done}"));
            done
        });

        self.line(format!("{function}({});", passed.join(", ")));

        self.gen.natives.borrow_mut().push((
            function.to_string(),
            format!("void {function}({});", parameters.join(", ")),
        ));

        let Some(done) = done else {
            return temp;
        };

        self.line(format!("{temp}.ok = {done}.ok;"));
        self.line(format!("{temp}.error = {done}.error;"));

        temp
    }
}

/// `ty` with every string and list full, as `unbounded` makes it, but each
/// `Bytes` in the size it has.
fn full_keeping_bytes(ty: &Type) -> Type {
    match ty {
        Type::SizedBytes(_) | Type::Bytes => ty.clone(),
        Type::Option(item) => Type::option(full_keeping_bytes(item)),
        Type::Result(value, error) => {
            Type::result(full_keeping_bytes(value), full_keeping_bytes(error))
        }
        other => unbounded(other),
    }
}

/// Whether a native giving back `ty` writes `Bytes`: `Bytes` itself, or a
/// `Result` of them.
pub fn gives_bytes(ty: &Type) -> bool {
    match ty {
        Type::Result(value, _) => is_bytes(value),
        ty => is_bytes(ty),
    }
}
