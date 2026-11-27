//! What a value read back from `Storage` must hold before a repository
//! gives it out. A save the program wrote always holds it, so one that
//! doesn't was damaged or changed outside the program, and loads as
//! `StorageDamaged`. The data comes from outside, so these checks are
//! written whatever the target's `checks` setting.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;
use croft_syntax::ast::ConstraintUse;

use crate::function::{Returns, Writer};
use crate::names::c_name;
use crate::types::bounds::{is_bytes, is_string};
use crate::types::width::declared_range;
use crate::Generator;

impl<'a> Generator<'a> {
    /// A C test, true when `number`, just read for a value of `ty` kept in
    /// the C type `c_type`, can be one: a flag is 0 or 1, a tag names a
    /// variant, an identity is in its `Range` and a narrow number fits its
    /// C type. `None` when every number can be one.
    pub(super) fn number_test(
        &self,
        number: &str,
        ty: &Type,
        c_type: Option<&str>,
    ) -> Option<String> {
        if let Some((low, high)) = c_type.and_then(c_type_bounds) {
            return Some(format!("{number} >= {low}L && {number} <= {high}L"));
        }

        match self.canonical(ty) {
            Type::Bool => Some(format!("{number} == 0 || {number} == 1")),
            Type::Named(named) if named.kind == SymbolKind::Enum => {
                Some(self.tag_test(number, &named))
            }
            Type::Named(named) if named.kind == SymbolKind::Identity => {
                let scope = &self.program.contexts[&named.context];
                let identity = scope
                    .identities
                    .iter()
                    .find(|i| i.name.name == named.name)?;
                let (low, high) = declared_range(
                    self.program,
                    &self.model,
                    &named.context,
                    &identity.constraints,
                )?;

                Some(format!("{number} >= {low}L && {number} <= {high}L"))
            }
            _ => None,
        }
    }

    /// A C test, true when `number` is the tag of one of the enum's variants.
    pub(super) fn tag_test(&self, number: &str, named: &Resolved) -> String {
        let count = self.variants(named).len();

        format!("{number} >= 0 && {number} < {count}")
    }

    /// A C test, true when `a` and `b`, two keys of `ty`, are the same.
    pub(super) fn same_key(&self, a: &str, b: &str, ty: &Type) -> String {
        if self.is_scalar(ty) {
            return format!("{a} == {b}");
        }

        if is_string(ty) {
            return format!("croft_string_equal(CROFT_VIEW({a}), CROFT_VIEW({b}))");
        }

        if is_bytes(ty) {
            return format!("croft_bytes_equal(CROFT_BYTES_VIEW({a}), CROFT_BYTES_VIEW({b}))");
        }

        self.equality_call(a, b, ty)
    }

    /// The function that tests a value of `named`, just read, against the
    /// constraints on its fields and an aggregate's invariants, and its
    /// prototype, when it has any. The name is `croft_kept_X`.
    pub(super) fn load_check(&self, named: &Resolved, name: &str) -> Option<(String, String)> {
        let ty = Type::Named(named.clone());
        let c = self.c_type(&ty);
        let signature = format!("static croft_bool croft_kept_{name}(const {c} *p_value)");

        let writer = if named.kind == SymbolKind::Enum {
            self.variant_checks(named)?
        } else {
            self.field_checks(named)?
        };

        let mut writer = writer;
        writer.line("return 1;");

        Some((format!("{signature};\n"), writer.finish(&signature)))
    }

    /// A writer that tests each field of a value, entity, aggregate or
    /// event against its constraints, and an aggregate against its
    /// invariants. `None` when there is nothing to test.
    fn field_checks(&self, named: &Resolved) -> Option<Writer<'_, 'a>> {
        let fields = self.model.fields(named);
        let invariants = self.invariants_of(named);

        let constrained = fields
            .iter()
            .any(|(_, _, field)| field.constraints.iter().any(|c| c.error.is_none()));

        if !constrained && invariants.is_empty() {
            return None;
        }

        let mut writer = self.load_writer(named);

        for (field, field_type, decl) in &fields {
            let mut place = format!("p_value->{field}");

            if let Some((byte, mask)) = self.flag_bit(named, field) {
                place = format!("((p_value->croft_flags[{byte}] & {mask}) != 0)");
            }

            for constraint in decl.constraints.iter().filter(|c| c.error.is_none()) {
                writer.require_constraint(&place, field_type, constraint);
            }
        }

        for invariant in &invariants {
            let Some(holds) = writer.invariant_holds(named, invariant, "(*p_value)") else {
                continue;
            };

            writer.open(format!("if (!{holds})"));
            writer.line("return 0;");
            writer.close();
        }

        Some(writer)
    }

    /// A writer that tests the fields of the variant an enum holds against
    /// their constraints. `None` when no variant's fields have any.
    fn variant_checks(&self, named: &Resolved) -> Option<Writer<'_, 'a>> {
        let mut writer = self.load_writer(named);
        let mut any = false;

        for variant in self.variants(named) {
            let declared = &variant.declared_in;

            let record = (self.program.contexts.get(&declared.context))
                .and_then(|context| context.enums.iter().find(|e| e.name.name == declared.name))
                .and_then(|decl| decl.variants.iter().find(|v| v.name.name == variant.name));

            let Some(record) = record else { continue };

            let constrained: Vec<(&String, &Type, &[ConstraintUse])> = (variant.fields.iter())
                .filter_map(|(field, ty)| {
                    let decl = record.fields.iter().find(|f| f.name.name == *field)?;
                    Some((field, ty, decl.constraints.as_slice()))
                })
                .filter(|(_, _, constraints)| constraints.iter().any(|c| c.error.is_none()))
                .collect();

            if constrained.is_empty() {
                continue;
            }

            any = true;
            writer.open(format!(
                "if (p_value->tag == {}_{})",
                c_name(named),
                variant.name
            ));

            for (field, ty, constraints) in constrained {
                let place = format!("p_value->as.{}.{field}", variant.name);

                for constraint in constraints.iter().filter(|c| c.error.is_none()) {
                    writer.require_constraint(&place, ty, constraint);
                }
            }

            writer.close();
        }

        any.then_some(writer)
    }

    /// The invariants of an aggregate, which every one the program makes or
    /// changes keeps. Other types have none that are kept.
    fn invariants_of(&self, named: &Resolved) -> Vec<croft_syntax::ast::Invariant> {
        if named.kind != SymbolKind::Aggregate {
            return Vec::new();
        }

        (self.program.contexts.get(&named.context))
            .and_then(|context| {
                context
                    .aggregates
                    .iter()
                    .find(|a| a.name.name == named.name)
            })
            .map_or(Vec::new(), |aggregate| aggregate.invariants.clone())
    }

    /// A writer for the body of a load check of `named`, looking up names
    /// where its declaration does.
    fn load_writer(&self, named: &Resolved) -> Writer<'_, 'a> {
        let mut writer = Writer::new(self, &named.context, Returns::Nothing);

        writer.aggregate = named
            .name
            .split('.')
            .next()
            .filter(|_| named.name.contains('.') || named.kind == SymbolKind::Aggregate)
            .map(str::to_string);

        writer.bind_parameters(&[("value".to_string(), Type::Named(named.clone()))]);

        writer
    }
}

/// The lowest and highest values a narrow C integer type holds.
fn c_type_bounds(c_type: &str) -> Option<(i64, i64)> {
    match c_type {
        "croft_u8" => Some((0, 255)),
        "croft_i8" => Some((-128, 127)),
        "croft_i16" => Some((-32768, 32767)),
        "croft_u16" => Some((0, 65535)),
        _ => None,
    }
}
