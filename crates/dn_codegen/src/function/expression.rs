//! Turning an expression into C: statements for anything that needs them,
//! and a C expression for the value.

use dn_analysis::{Meaning, Type};
use dn_syntax::ast::{Expr, ExprKind, UnaryOp};

use super::Writer;
use crate::names::c_string;
use crate::types::constant::{scale, scaled};

impl<'g, 'a> Writer<'g, 'a> {
    /// The value of `value`, as a C expression of the C type for `want`.
    pub fn value(&mut self, value: &Expr, want: &Type) -> String {
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

    /// The value of an expression as a C expression, and its type.
    pub fn eval(&mut self, value: &Expr, want: Option<&Type>) -> (String, Type) {
        let ty = self.type_of(value, want);
        let c = match &value.kind {
            ExprKind::Int(number) => format!("{number}L"),
            ExprKind::Float(text) => match ty {
                Type::Decimal(places) => format!("{}L", scaled(text, places)),
                _ => text.clone(),
            },
            ExprKind::Bool(flag) => (if *flag { "1" } else { "0" }).to_string(),
            ExprKind::String(text) => {
                let temp = self.temp(&Type::String);
                self.line(format!("dn_string_set(&{temp}, {});", c_string(text)));
                temp
            }
            ExprKind::Name(name) => self.name(value, name, &ty),
            ExprKind::Field(inner, field) => match self.gen.facts.meaning(value.id) {
                Some(Meaning::Property) => self.property(inner, &field.name),
                _ => self.name(value, &field.name, &ty),
            },
            ExprKind::Call(callee, arguments) => self.call(value, callee, arguments, &ty),
            ExprKind::List(items) => self.list_literal(items, &ty),
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
    /// into a successful `Result`. Every error is already `dn_error`.
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

    /// A single-value type around `value`, checked against its constraints
    /// in debug builds: `Score(120)`, or `120` where a `Score` is expected.
    pub fn wrap_single_value(&mut self, value: String, ty: &Type) -> String {
        let temp = self.temp(ty);
        self.line(format!("{temp}.value = {value};"));
        self.check_fields(&temp, ty);
        temp
    }

    /// A C expression that is true when two values of `ty` are equal.
    pub fn equal(&mut self, left: &str, right: &str, ty: &Type) -> String {
        if self.gen.is_scalar(ty) {
            return format!("({left} == {right})");
        }
        if *ty == Type::String {
            return format!("dn_string_equal(&{left}, &{right})");
        }
        if *ty == Type::Bytes {
            return format!("dn_bytes_equal(&{left}, &{right})");
        }
        let name = self.gen.equality_for(ty);
        format!("{name}(&{left}, &{right})")
    }
}

fn has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(item) | Type::Option(item) => has_unknown(item),
        Type::Result(value, error) => has_unknown(value) || has_unknown(error),
        _ => false,
    }
}
