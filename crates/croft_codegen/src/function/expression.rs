//! Turning an expression into C: statements for anything that needs them,
//! and a C expression for the value.

use croft_analysis::{Meaning, Type};
use croft_syntax::ast::{Expr, ExprKind, UnaryOp};

use super::Writer;
use crate::names::c_string;
use crate::types::bounds::{is_string, unbounded};
use crate::types::constant::{scale, scaled};

impl<'g, 'a> Writer<'g, 'a> {
    /// The value of `value`, as a C expression of the C type for `want`.
    pub fn value(&mut self, value: &Expr, want: &Type) -> String {
        // A literal, `[]` or `None` is written straight into the type wanted.
        if matches!(value.kind, ExprKind::String(_)) && is_string(want)
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

    fn eval_unbounded(&mut self, value: &Expr, want: Option<&Type>) -> (String, Type) {
        let ty = self.type_of(value, want);

        let c = match &value.kind {
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
            ExprKind::Call(callee, arguments) => self.call(value, callee, arguments, &ty),
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
            ExprKind::Unary(UnaryOp::Negate, operand) => format!("(-{})", self.value(operand, &ty)),
            ExprKind::Unary(UnaryOp::BitNot, operand) => {
                format!("(~{})", self.value(operand, &Type::Int))
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
            ExprKind::Issue(issue) => self.issue(value, issue, &ty),
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

    /// Converts a value to another type with the same meaning: a plain value
    /// into a successful `Result`. Every error is already `croft_error`.
    pub fn convert(&mut self, c: String, from: &Type, to: &Type) -> String {
        if let (Type::Decimal(places), Type::Int) = (to, from) {
            return format!("({c} * {}L)", scale(*places));
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
            match ty {
                Type::List(_) => self.line(format!("{target}.length = 0;")),
                _ => self.line(format!("croft_clear(&{target}, sizeof {target});")),
            }

            let empty = self.gen.narrowest(ty);
            self.note_result(target, &empty);
            return;
        }

        let (c, from) = self.eval(value, Some(ty));

        if unbounded(&from) == unbounded(ty) {
            self.copy_into(target, &c, &from, ty);
        } else {
            let c = self.convert(c, &from, ty);
            self.note_result(target, ty);
            self.line(format!("{target} = {c};"));
        }
    }

    /// Whether `value` is `[]` for a list of `ty`, or `None` for an option.
    fn is_empty(&self, value: &Expr, ty: &Type) -> bool {
        match (&value.kind, ty) {
            (ExprKind::List(items), Type::List(_)) => items.is_empty(),
            (ExprKind::Name(_) | ExprKind::Field(..), Type::Option(_)) => {
                matches!(self.gen.facts.meaning(value.id), Some(Meaning::NoneValue))
            }
            _ => false,
        }
    }

    /// Writes `source`, of type `from`, into `target`, of type `to`, which
    /// differ at most in the bounds of their strings. A string longer than
    /// `to` has room for is cut short, and stops a debug build.
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
            (Type::List(from), Type::List(to)) => {
                let index = self.variable("index", &Type::Int);
                self.line(format!("{target}.length = {source}.length;"));

                self.open(format!(
                    "for ({index} = 0; {index} < (croft_int) {source}.length; {index}++)"
                ));

                let (target, source) = (
                    format!("{target}.items[{index}]"),
                    format!("{source}.items[{index}]"),
                );

                self.copy_into(&target, &source, from, to);
                self.close();
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
                let index = self.variable("index", &Type::Int);
                self.line(format!("{target}.length = {source}.length;"));

                self.open(format!(
                    "for ({index} = 0; {index} < (croft_int) {source}.length; {index}++)"
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

    /// A single-value type around `value`, checked against its constraints
    /// in debug builds: `Score(120)`, or `120` where a `Score` is expected.
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

        if *ty == Type::Bytes {
            return format!("croft_bytes_equal(&{left}, &{right})");
        }

        let name = self.gen.equality_for(ty);

        format!("{name}(&{left}, &{right})")
    }
}

fn has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(item) | Type::Option(item) => has_unknown(item),
        Type::Map(key, value) => has_unknown(key) || has_unknown(value),
        Type::Result(value, error) => has_unknown(value) || has_unknown(error),
        _ => false,
    }
}
