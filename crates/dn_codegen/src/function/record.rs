//! Records, `with` and enum variants with data: setting fields on a temporary.

use dn_analysis::types::Resolved;
use dn_analysis::Type;
use dn_syntax::ast::{Argument, FieldValue};

use super::Writer;
use crate::names::{c_name, error_code};

impl<'g, 'a> Writer<'g, 'a> {
    /// `Position { x: 10, y: 20 }`, or an error with fields.
    pub fn record(&mut self, ty: &Type, fields: &[FieldValue]) -> String {
        if self.gen.model.is_error(ty) {
            let Type::Named(error) = ty else {
                return "0".to_string();
            };
            let temp = self.error_value(error);
            let prefix = format!("{temp}.fields.{}", c_name(error));
            self.set_named_fields(&prefix, None, &self.field_types(error), fields);
            return temp;
        }
        let temp = self.temp(ty);
        self.set_fields(&temp, ty, fields);
        self.check_fields(&temp, ty);
        temp
    }

    /// Sets each given field of `target`, a value of type `ty`.
    pub fn set_fields(&mut self, target: &str, ty: &Type, fields: &[FieldValue]) {
        let Type::Named(named) = ty else { return };
        let types = self.field_types(named);
        self.set_named_fields(target, Some(named), &types, fields);
    }

    pub fn field_types(&self, named: &Resolved) -> Vec<(String, Type)> {
        self.gen
            .struct_fields(named)
            .into_iter()
            .map(|(name, ty, _)| (name, ty))
            .collect()
    }

    fn set_named_fields(
        &mut self,
        target: &str,
        owner: Option<&Resolved>,
        types: &[(String, Type)],
        fields: &[FieldValue],
    ) {
        for given in fields {
            let name = &given.name.name;
            let field_type = types
                .iter()
                .find(|(f, _)| f == name)
                .map_or(Type::Unknown, |(_, ty)| ty.clone());
            let value = match &given.value {
                Some(value) => self.value(value, &field_type),
                None => self.read_name(name),
            };
            self.write_field(owner, target, name, &value);
        }
    }

    /// `TreasureKind.Coins(count: 20)`: a variant with data, or an error enum's own variant.
    pub fn variant_call(
        &mut self,
        owner: &Resolved,
        variant: &str,
        error: Option<&Resolved>,
        arguments: &[Argument],
        ty: &Type,
    ) -> String {
        let info = self
            .gen
            .model
            .variants(owner)
            .into_iter()
            .find(|v| v.name == variant);
        let Some(info) = info else {
            return "0".to_string();
        };
        let names: Vec<String> = info.fields.iter().map(|(name, _)| name.clone()).collect();
        let values = self.ordered(arguments, &names);
        let owner_is_error = self.gen.model.is_error(&Type::Named(owner.clone()));
        let (temp, prefix) = if error.is_some() || owner_is_error {
            let error = error
                .cloned()
                .unwrap_or_else(|| self.gen.own_error(owner, &info));
            let temp = self.temp(&Type::Errors(Vec::new()));
            self.line(format!("{temp}.code = {};", error_code(&error)));
            let prefix = format!("{temp}.fields.{}", c_name(&error));
            (temp, prefix)
        } else {
            let temp = self.temp(ty);
            self.line(format!("{temp}.tag = {}_{variant};", c_name(owner)));
            let prefix = format!("{temp}.as.{variant}");
            (temp, prefix)
        };
        for ((name, field_type), value) in info.fields.iter().zip(values) {
            let Some(value) = value else { continue };
            let value = self.value(value, field_type);
            self.line(format!("{prefix}.{name} = {value};"));
        }
        temp
    }
}
