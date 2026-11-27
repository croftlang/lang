//! Turning an expression into C: statements for anything that needs them,
//! and a C expression for the value.

use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Expr, ExprKind, UnaryOp};

use super::range::{c_number, WHOLE_LOW};
use super::Writer;
use crate::names::c_string;
use crate::types::bounds::{is_bytes, is_string, list_item, unbounded};
use crate::types::constant::{evaluate, scale, scaled};

impl<'g, 'a> Writer<'g, 'a> {
    /// The value of `value`, as a C expression of the C type for `want`.
    pub fn value(&mut self, value: &Expr, want: &Type) -> String {
        // A value made for a place of its own owns its strings.
        let want = &self.gen.owned_type(want);

        // A literal, `[]` or `None` is written straight into the type wanted.
        if matches!(value.kind, ExprKind::String(_)) && is_string(want)
            || matches!(value.kind, ExprKind::List(_)) && list_item(want).is_some()
            || self.is_empty(value, want)
        {
            let temp = self.temp(want);
            self.assign(&temp, value, want);
            return temp;
        }

        let (c, ty) = self.eval(value, Some(want));
        self.convert(c, &ty, want)
    }

    /// The type the checker gave an expression, made concrete by `want` where
    /// the checker could not know it, as with `None`.
    pub fn type_of(&self, value: &Expr, want: Option<&Type>) -> Type {
        let ty = self.gen.facts.type_of(value.id).clone();

        match want {
            Some(want) if has_unknown(&ty) => want.clone(),
            _ => ty,
        }
    }

    /// The value of an expression as a C expression, and its type, with the
    /// bounds of any strings it holds.
    pub fn eval(&mut self, value: &Expr, want: Option<&Type>) -> (String, Type) {
        let (c, ty) = self.eval_unbounded(value, want);
        let ty = self.bounded_type(&c, &ty);

        (c, ty)
    }

    /// The value of an expression as a C expression, and the type the
    /// checker gave it, without the bounds of its strings.
    fn eval_unbounded(&mut self, value: &Expr, want: Option<&Type>) -> (String, Type) {
        let ty = self.type_of(value, want);

        let c = match &value.kind {
            ExprKind::Int(number) if ty == Type::Int => c_number(*number),
            ExprKind::Int(number) => format!("{number}L"),
            ExprKind::Float(text) => match ty {
                Type::Decimal(places) => format!("{}L", scaled(text, places)),
                _ => text.clone(),
            },
            ExprKind::Bool(flag) => (if *flag { "1" } else { "0" }).to_string(),
            ExprKind::String(text) => {
                let literal = self.gen.text_type(text.chars().count() as i64);
                let temp = self.temp(&literal);
                self.line(format!(
                    "croft_string_set({}, CROFT_OUT({temp}));",
                    c_string(text)
                ));
                temp
            }
            ExprKind::Name(name) => self.name(value, name, &ty),
            ExprKind::Field(inner, field) => match self.gen.facts.meaning(value.id) {
                Some(Meaning::Property) => self.property(inner, &field.name),
                _ => self.name(value, &field.name, &ty),
            },
            ExprKind::Call(callee, arguments) => self.call(value, callee, arguments, &ty, None),
            ExprKind::List(items) => self.list_literal(items, &ty),
            ExprKind::Map(entries) => self.map_literal(entries, &ty),
            ExprKind::Record(_, fields) => self.record(&ty, fields),
            ExprKind::With(base, fields) => {
                let base = self.value(base, &ty);
                let temp = self.temp(&ty);
                self.line(format!("{temp} = {base};"));
                self.set_fields(&temp, &ty, fields);
                self.check_fields(&temp, &ty);
                temp
            }
            ExprKind::Unary(UnaryOp::Not, operand) => {
                format!("(!{})", self.value(operand, &Type::Bool))
            }
            ExprKind::Unary(UnaryOp::Negate, operand) => self.negate(value, operand, &ty),
            ExprKind::Unary(UnaryOp::BitNot, operand) => {
                let inner = self.value(operand, &Type::Int);
                format!("(~{})", self.widened(inner, value, operand))
            }
            ExprKind::Binary(op, left, right) => self.binary(*op, left, right, &ty),
            ExprKind::OrElse(inner, error) => self.or_else(inner, error, &ty),
            ExprKind::Try(inner) => self.try_value(inner),
            ExprKind::Match(subject, arms) => self.check_match(subject, arms, &ty),
            ExprKind::Block(statements) => self.block(statements, &ty),
            ExprKind::If(condition, block) => {
                self.guard(condition, block);
                "0".to_string()
            }
            ExprKind::Issue(issue) => self.issue(value, issue, &ty, None),
            ExprKind::Emit(event, fields) => self.emit(event, fields),
            ExprKind::Native(function, arguments) => self.native(&function.name, arguments, want),
            ExprKind::Loop(counter, block) => self.repeat(counter.as_ref(), block),
            ExprKind::Break => {
                self.line("break;");
                "0".to_string()
            }
            ExprKind::Lambda(..) => "0".to_string(),
        };

        (c, ty)
    }

    /// `-operand`, which goes past 32 bits only for the lowest whole number.
    /// A constant is written as its value, so the lowest can be written at
    /// all; one whose range leaves out the lowest is plain C.
    fn negate(&mut self, whole: &Expr, operand: &Expr, ty: &Type) -> String {
        if *ty == Type::Int {
            if let Some(constant) =
                evaluate(self.gen.program, &self.gen.model, &self.context, whole)
            {
                return c_number(constant);
            }
        }

        let inner = self.value(operand, ty);

        match self.range_of(operand) {
            Some(range) if *ty == Type::Int && range.low > WHOLE_LOW => {
                format!("(-{})", self.widened(inner, whole, operand))
            }
            _ if matches!(ty, Type::Decimal(_)) => format!("croft_decimal_negate({inner})"),
            _ => format!("croft_int_negate({inner})"),
        }
    }

    /// Converts a value to another type with the same meaning: a plain value
    /// into a successful `Result`. Every error is already `croft_error`.
    pub fn convert(&mut self, c: String, from: &Type, to: &Type) -> String {
        if let (Type::Decimal(places), Type::Int) = (to, from) {
            return format!("croft_decimal_scale({c}, {}L)", scale(*places));
        }

        if let Some(inner) = self.gen.model.single_value_inner(to) {
            if inner != *to && self.gen.c_type(from) != self.gen.c_type(to) {
                let value = self.convert(c, from, &inner);
                return self.wrap_single_value(value, to);
            }
        }

        if self.gen.c_type(from) == self.gen.c_type(to) {
            return c;
        }

        if unbounded(from) == unbounded(to) {
            let temp = self.temp(to);
            self.copy_into(&temp, &c, from, to);
            return temp;
        }

        match (to, from) {
            (Type::Result(value, _), from) if !matches!(from, Type::Result(..)) => {
                let inner = self.convert(c, from, value);
                let temp = self.temp(to);
                self.line(format!("{temp}.ok = 1;"));
                self.line(format!("{temp}.value = {inner};"));
                temp
            }
            _ => c,
        }
    }

    /// Works out `value` and writes it into `target`, a place of type `ty`.
    pub fn assign(&mut self, target: &str, value: &Expr, ty: &Type) {
        if let (ExprKind::String(text), true) = (&value.kind, is_string(ty)) {
            self.line(format!(
                "croft_string_set({}, CROFT_OUT({target}));",
                c_string(text)
            ));

            let literal = self.gen.text_type(text.chars().count() as i64);
            self.note_result(target, &literal);

            let room = self.gen.bound(ty);

            if text.chars().count() as i64 > room {
                self.line(format!(
                    "CROFT_CHECK(0, \"text is longer than {room} characters\");"
                ));
            }

            return;
        }

        if self.is_empty(value, ty) {
            match list_item(ty) {
                Some(_) => self.line(format!("{target}.length = 0;")),
                None => self.line(format!("croft_clear(&{target}, sizeof {target});")),
            }

            let empty = self.gen.narrowest(ty);
            self.note_result(target, &empty);
            return;
        }

        if let (ExprKind::List(items), Some(item)) = (&value.kind, list_item(ty)) {
            let literal = self.gen.list_type(item.clone(), items.len() as i64);
            self.fill_list(target, items, ty);
            self.note_result(target, &literal);
            return;
        }

        let (c, from) = self.eval(value, Some(ty));
        self.store(target, c, &from, ty);
    }

    /// Writes `c`, a value of type `from`, into `target`, a place of type `ty`.
    pub fn store(&mut self, target: &str, c: String, from: &Type, ty: &Type) {
        if c == target {
            return;
        }

        if unbounded(from) == unbounded(ty) {
            self.copy_into(target, &c, from, ty);
        } else {
            let c = self.convert(c, from, ty);
            self.note_result(target, ty);
            self.line(format!("{target} = {c};"));
        }
    }

    /// Whether `value` is `[]` for a list of `ty`, or `None` for an option.
    fn is_empty(&self, value: &Expr, ty: &Type) -> bool {
        match (&value.kind, ty) {
            (ExprKind::List(items), Type::List(_) | Type::SizedList(..)) => items.is_empty(),
            (ExprKind::Name(_) | ExprKind::Field(..), Type::Option(_)) => {
                matches!(self.gen.facts.meaning(value.id), Some(Meaning::NoneValue))
            }
            _ => false,
        }
    }

    /// Writes `source`, of type `from`, into `target`, of type `to`, which
    /// differ at most in the bounds of their strings, `Bytes` and lists. A
    /// string, `Bytes` or list longer than `to` has room for is cut short,
    /// and stops a checked build.
    pub fn copy_into(&mut self, target: &str, source: &str, from: &Type, to: &Type) {
        self.note_result(target, from);

        if self.gen.c_type(from) == self.gen.c_type(to) {
            self.line(format!("{target} = {source};"));
            return;
        }

        match (from, to) {
            (from, to) if is_string(from) && is_string(to) => {
                let view = self.view_of(source);
                let room = self.gen.bound(to);

                if self.gen.bound(from) <= room {
                    self.line(format!("croft_string_copy({view}, CROFT_OUT({target}));"));
                    return;
                }

                self.open(format!(
                    "if (!croft_string_copy({view}, CROFT_OUT({target})))"
                ));
                self.line(format!(
                    "CROFT_CHECK(0, \"text is longer than {room} characters\");"
                ));

                self.close();
            }
            (from, to) if is_bytes(from) && is_bytes(to) => {
                let view = format!("CROFT_BYTES_VIEW({source})");
                let room = self.gen.bound(to);

                if self.gen.bound(from) <= room {
                    self.line(format!(
                        "croft_bytes_copy({view}, CROFT_BYTES_OUT({target}));"
                    ));
                    return;
                }

                self.open(format!(
                    "if (!croft_bytes_copy({view}, CROFT_BYTES_OUT({target})))"
                ));
                self.line(format!("CROFT_CHECK(0, \"bytes hold at most {room}\");"));

                self.close();
            }
            (from_list, to_list)
                if list_item(from_list).is_some() && list_item(to_list).is_some() =>
            {
                self.copy_list(target, source, from_list, to_list);
            }
            (Type::Option(from), Type::Option(to)) => {
                self.line(format!("{target}.some = {source}.some;"));

                self.open(format!("if ({source}.some)"));
                let (target, source) = (format!("{target}.value"), format!("{source}.value"));
                self.copy_into(&target, &source, from, to);
                self.close();
            }
            (Type::Result(from, from_error), Type::Result(to, to_error)) => {
                self.line(format!("{target}.ok = {source}.ok;"));

                self.open(format!("if ({source}.ok)"));
                let (value, given) = (format!("{target}.value"), format!("{source}.value"));
                self.copy_into(&value, &given, from, to);
                self.close();

                self.open("else");
                let (error, given) = (format!("{target}.error"), format!("{source}.error"));
                self.copy_into(&error, &given, from_error, to_error);
                self.close();
            }
            (Type::Map(from_key, from_value), Type::Map(to_key, to_value)) => {
                let index = self.index_variable();
                self.line(format!("{target}.length = {source}.length;"));

                self.open(format!(
                    "for ({index} = 0; {index} < {source}.length; {index}++)"
                ));

                for (part, from, to) in
                    [("keys", from_key, to_key), ("values", from_value, to_value)]
                {
                    let (target, source) = (
                        format!("{target}.{part}[{index}]"),
                        format!("{source}.{part}[{index}]"),
                    );

                    self.copy_into(&target, &source, from, to);
                }

                self.close();
            }
            _ => self.line(format!("{target} = {source};")),
        }
    }

    /// Copies a list item by item into one of another size, or with items
    /// of other bounds. Items past the room `to` has are left out, and stop
    /// a checked build.
    fn copy_list(&mut self, target: &str, source: &str, from: &Type, to: &Type) {
        let (Some(from_item), Some(to_item)) = (list_item(from), list_item(to)) else {
            return;
        };

        let (from_item, to_item) = (from_item.clone(), to_item.clone());
        let room = self.gen.capacity(to);
        let index = self.index_variable();

        let length = if self.gen.capacity(from) <= room {
            format!("{source}.length")
        } else {
            self.line(format!(
                "CROFT_CHECK({source}.length <= {room}, \"a list holds at most {room} items\");"
            ));

            format!("({source}.length < {room} ? {source}.length : {room})")
        };

        self.line(format!("{target}.length = {length};"));

        self.open(format!(
            "for ({index} = 0; {index} < {target}.length; {index}++)"
        ));

        let (target, source) = (
            format!("{target}.items[{index}]"),
            format!("{source}.items[{index}]"),
        );

        self.copy_into(&target, &source, &from_item, &to_item);
        self.close();
    }

    /// A single-value type around `value`, checked against its constraints
    /// when checks are on: `Score(120)`, or `120` where a `Score` is expected.
    pub fn wrap_single_value(&mut self, value: String, ty: &Type) -> String {
        let temp = self.temp(ty);

        let owner = match ty {
            Type::Named(owner) => Some(owner.clone()),
            _ => None,
        };

        self.write_field(owner.as_ref(), &temp, "value", &value);
        self.check_fields(&temp, ty);

        temp
    }

    /// A C expression that is true when two values of `ty` are equal.
    pub fn equal(&mut self, left: &str, right: &str, ty: &Type) -> String {
        if self.gen.is_scalar(ty) {
            return format!("({left} == {right})");
        }

        if is_string(ty) {
            return format!(
                "croft_string_equal({}, {})",
                self.view_of(left),
                self.view_of(right)
            );
        }

        if is_bytes(ty) {
            return format!(
                "croft_bytes_equal(CROFT_BYTES_VIEW({left}), CROFT_BYTES_VIEW({right}))"
            );
        }

        self.gen.equality_call(left, right, ty)
    }
}

/// Whether `ty` has a type the checker could not know anywhere in it.
fn has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(item) | Type::SizedList(item, _) | Type::Option(item) => has_unknown(item),
        Type::Map(key, value) => has_unknown(key) || has_unknown(value),
        Type::Result(value, error) => has_unknown(value) || has_unknown(error),
        _ => false,
    }
}
