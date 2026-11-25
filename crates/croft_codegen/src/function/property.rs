//! Names and properties: locals, own fields, constants, errors, variants,
//! and `value.field`.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{Meaning, Type};
use croft_syntax::ast::Expr;

use super::Writer;
use crate::names::{c_name, error_code};

impl<'g, 'a> Writer<'g, 'a> {
    pub fn name(&mut self, value: &Expr, name: &str, ty: &Type) -> String {
        match self.gen.facts.meaning(value.id).cloned() {
            Some(Meaning::Const(constant)) => c_name(&constant),
            Some(Meaning::Error(error)) => self.error_value(&error),
            Some(Meaning::Variant(owner, variant, error)) => {
                self.variant_value(&owner, &variant, error.as_ref(), ty)
            }
            Some(Meaning::Unit) => "0".to_string(),
            Some(Meaning::NoneValue) => {
                let temp = self.temp(ty);
                self.line(format!("{temp}.some = 0;"));
                temp
            }
            _ => self.read_name(name),
        }
    }

    /// A local name's value, or an own field's, read as a full string when
    /// it is kept in its own size.
    pub fn read_name(&mut self, name: &str) -> String {
        if self.lookup(name).is_some() {
            return self.local_or_field(name);
        }

        let Some(own) = &self.own else {
            return self.local_or_field(name);
        };

        let (prefix, owner) = (
            own.prefix.trim_end_matches('.').to_string(),
            own.owner.clone(),
        );

        self.read_field(owner.as_ref(), &prefix, name)
    }

    /// A local name, or else a field of the command's aggregate or the adapter.
    pub fn local_or_field(&self, name: &str) -> String {
        if let Some((c, _)) = self.lookup(name) {
            return c;
        }

        match &self.own {
            Some(own) => format!("{}{name}", own.prefix),
            None => name.to_string(),
        }
    }

    /// An error with no fields, as a `croft_error` value.
    pub fn error_value(&mut self, error: &Resolved) -> String {
        let temp = self.temp(&Type::Errors(Vec::new()));
        self.line(format!("{temp}.code = {};", error_code(error)));
        temp
    }

    /// A variant without fields: a number for a plain enum, otherwise a
    /// struct with its tag set, or an error.
    fn variant_value(
        &mut self,
        owner: &Resolved,
        variant: &str,
        error: Option<&Resolved>,
        ty: &Type,
    ) -> String {
        let as_error = error.cloned().or_else(|| {
            let owner_type = Type::Named(owner.clone());

            self.gen.model.is_error(&owner_type).then(|| Resolved {
                context: owner.context.clone(),
                name: format!("{}.{variant}", owner.name),
                kind: SymbolKind::Error,
            })
        });

        if let Some(error) = as_error {
            return self.error_value(&error);
        }

        let tag = format!("{}_{variant}", c_name(owner));

        if self.gen.is_plain_enum(owner) {
            return tag;
        }

        let temp = self.temp(ty);
        self.line(format!("{temp}.tag = {tag};"));

        temp
    }

    pub fn property(&mut self, inner: &Expr, field: &str) -> String {
        let inner_type = self.type_of(inner, None);
        let (c, _) = self.eval(inner, None);

        match (&inner_type, field) {
            (Type::String | Type::Bytes | Type::List(_), "length") => {
                format!("((croft_int) {c}.length)")
            }
            (Type::String | Type::Bytes | Type::List(_), "is_empty") => {
                format!("({c}.length == 0)")
            }
            (Type::Map(..), "length") => format!("((croft_int) {c}.length)"),
            (Type::Map(..), "is_empty") => format!("({c}.length == 0)"),
            (Type::Map(key, value), "keys" | "values") => {
                let item = if field == "keys" { key } else { value };
                self.map_list(&c, item, field)
            }
            (Type::List(item), "first" | "last") => {
                let temp = self.temp(&Type::option((**item).clone()));

                let index = if field == "first" {
                    "0".to_string()
                } else {
                    format!("{c}.length - 1")
                };

                self.line(format!("{temp}.some = {c}.length > 0;"));
                self.open(format!("if ({temp}.some)"));
                self.line(format!("{temp}.value = {c}.items[{index}];"));
                self.close();

                temp
            }
            (Type::Option(_), "is_some") => format!("{c}.some"),
            (Type::Option(_), "is_none") => format!("(!{c}.some)"),
            (Type::Named(owner), _) => {
                let owner = owner.clone();
                self.read_field(Some(&owner), &c, field)
            }
            _ => format!("{c}.{field}"),
        }
    }
}
