//! Repositories. Each aggregate is turned into bytes and kept under a key,
//! such as `rooms12` for room 12, by the adapter for `Storage` that croft.toml
//! binds to the repository: `MemoryStorage` unless it names another. Or an
//! adapter written for the repository keeps them its own way. A
//! saved value starts with the hash of the aggregate's layout, so one written
//! by a different version of the program loads as `StorageDamaged`. Saving
//! keeps the events the aggregate's commands raised, so policies hear of them.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::{StorageShape, Type};
use croft_syntax::ast::ConstraintUse;

use super::constraints::CheckPlace;
use super::layout::layout_hash;
use super::serialize::Direction;
use crate::names::{aggregate_kind, c_name, c_string, error_code};
use crate::Generator;

/// Keys fit the eight characters of a FAT file name.
const KEY_LENGTH: usize = 8;

/// The repository, aggregate and `Storage` adapter that one operation
/// `Storage` provides works on.
struct Stored<'r> {
    repository: &'r Resolved,
    aggregate: &'r Resolved,
    storage: &'r Resolved,
    /// Its one parameter: the aggregate or its identity.
    parameter: &'r (String, Type),
    /// The check of that parameter against its constraints when checks are
    /// on, written after the declarations, or nothing when it has none.
    check: String,
}

impl<'a> Generator<'a> {
    /// Each repository operation as a C function, through the adapter
    /// croft.toml binds: written for the repository itself, or for `Storage`.
    pub fn repositories(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();

        for repository in self.model.all(SymbolKind::Repository) {
            let Some(aggregate) = self.model.repository_aggregate(&repository) else {
                continue;
            };

            let Some(storage) = self.storages.get(&repository) else {
                continue;
            };

            let handwritten = self.is_repository_adapter(&repository, storage);

            for operation in self.model.operations_of(&repository) {
                let parameters: Vec<(String, Type)> = (operation.parameters.iter())
                    .map(|(name, ty, _)| (name.clone(), ty.clone()))
                    .collect();

                let function = format!("{}_{}", c_name(&repository), operation.name);
                let signature =
                    self.signature(&function, &[], &parameters, Some(&operation.result));
                prototypes.push(format!("{signature};"));

                let shape = self.model.storage_shape(&repository, &operation.name);

                let body = if handwritten {
                    self.forwarded(&aggregate, storage, &operation.name, &parameters, shape)
                } else {
                    let Some(shape) = shape else {
                        continue;
                    };

                    // An adapter written for the repository checks the
                    // arguments in its own operation, so only these need it here.
                    let label = format!("{}.{}", repository.name, operation.name);

                    let place = CheckPlace {
                        context: &repository.context,
                        aggregate: None,
                        label: &label,
                    };

                    let checked: Vec<(String, Type, &[ConstraintUse])> = (operation.parameters)
                        .iter()
                        .map(|(name, ty, constraints)| (name.clone(), ty.clone(), *constraints))
                        .collect();

                    let check = self
                        .argument_check(&function, &place, &checked)
                        .map_or(String::new(), |call| {
                            format!("#ifdef CROFT_CHECKS\n    {call}\n#endif\n")
                        });

                    let stored = Stored {
                        repository: &repository,
                        aggregate: &aggregate,
                        storage,
                        parameter: &parameters[0],
                        check,
                    };

                    self.stored(&stored, shape)
                };

                out.push_str(&format!("{signature}\n{{\n{body}}}\n\n"));
            }
        }

        out
    }

    /// How C reads a parameter: by value, or through its pointer.
    fn parameter_value(&self, (name, ty): &(String, Type)) -> String {
        if self.is_scalar(ty) {
            format!("p_{name}")
        } else {
            format!("(*p_{name})")
        }
    }

    /// An operation of an adapter written for the repository, called as it
    /// is. A save still keeps the aggregate's events.
    fn forwarded(
        &self,
        aggregate: &Resolved,
        adapter: &Resolved,
        operation: &str,
        parameters: &[(String, Type)],
        shape: Option<StorageShape>,
    ) -> String {
        let mut passed: Vec<String> = parameters
            .iter()
            .map(|(name, _)| format!("p_{name}"))
            .collect();

        passed.push("out".to_string());

        let call = format!(
            "    {}_{operation}({});\n",
            c_name(adapter),
            passed.join(", ")
        );

        if shape != Some(StorageShape::Save) {
            return call;
        }

        let field = self.identity_field(aggregate);
        let saved = format!("{}.{field}", self.parameter_value(&parameters[0]));
        let kind = aggregate_kind(aggregate);
        let key = self.event_key(aggregate, &saved);

        format!("{call}    if (out->ok) {{\n        croft_events_keep({kind}, {key});\n    }}\n")
    }

    /// An operation `Storage` provides: the aggregate turned into bytes and
    /// kept under a key made from its identity.
    fn stored(&self, stored: &Stored, shape: StorageShape) -> String {
        let Stored {
            repository,
            aggregate,
            storage,
            parameter,
            ref check,
        } = *stored;

        let aggregate_type = Type::Named(aggregate.clone());
        let hash = layout_hash(&self.layout(&aggregate_type));
        let field = self.identity_field(aggregate);
        let prefix = c_string(&self.key_prefix(repository, aggregate));
        let storage = c_name(storage);
        let value = self.parameter_value(parameter);

        let loaded = self.c_type(&self.loaded_type());

        let written = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));

        let error = |name: &str| {
            self.model
                .lookup("Std.Storage", name)
                .map_or("0".to_string(), |error| error_code(&error))
        };

        let (full, damaged) = (error("StorageFull"), error("StorageDamaged"));

        match shape {
            StorageShape::Find | StorageShape::FindOption => {
                let serializer = self.serializer_for(&aggregate_type, Direction::Read);

                let (missing, into, found) = if shape == StorageShape::Find {
                    let not_found = self
                        .model
                        .lookup(&aggregate.context, &format!("{}.NotFound", aggregate.name))
                        .map_or("0".to_string(), |error| error_code(&error));

                    (
                        format!("out->error.code = {not_found};\n        return;"),
                        "out->value",
                        "",
                    )
                } else {
                    (
                        "out->ok = 1;\n        return;".to_string(),
                        "out->value.value",
                        "\n    out->value.some = 1;",
                    )
                };

                format!(
                    "    croft_string key;\n    {loaded} kept;\n    croft_u16 at = 0;\n    croft_int layout = 0;\n\n{check}    \
                     croft_clear(out, sizeof *out);\n    croft_storage_key({prefix}, (croft_int) {value}, &key);\n    {storage}_load(CROFT_VIEW(key), &kept);\n    \
                     if (!kept.ok) {{\n        out->error = kept.error;\n        return;\n    }}\n    \
                     if (!kept.value.some) {{\n        {missing}\n    }}\n    \
                     if (!croft_bytes_take_int(CROFT_BYTES_VIEW(kept.value.value), &at, &layout) || layout != {hash}L\n        \
                     || !croft_read_{serializer}(CROFT_BYTES_VIEW(kept.value.value), &at, &{into}) || at != kept.value.value.length) {{\n        \
                     croft_clear(out, sizeof *out);\n        out->error.code = {damaged};\n        return;\n    }}{found}\n    out->ok = 1;\n"
                )
            }
            StorageShape::Save => {
                let serializer = self.serializer_for(&aggregate_type, Direction::Write);
                let kind = aggregate_kind(aggregate);
                let key = self.event_key(aggregate, &format!("{value}.{field}"));

                // Room for this aggregate's save, however large a `Bytes`
                // elsewhere in the program can be.
                let data = self.c_type(&self.saved_bytes(aggregate));

                format!(
                    "    croft_string key;\n    {data} data;\n    croft_bytes_writer writer;\n    {written} kept;\n\n{check}    \
                     croft_clear(out, sizeof *out);\n    writer = croft_bytes_writer_to(CROFT_BYTES_OUT(data));\n    \
                     if (!croft_bytes_put_int(&writer, {hash}L) || !croft_write_{serializer}(&{value}, &writer)) {{\n        \
                     out->error.code = {full};\n        return;\n    }}\n    \
                     croft_storage_key({prefix}, (croft_int) {value}.{field}, &key);\n    {storage}_save(CROFT_VIEW(key), CROFT_BYTES_VIEW(data), &kept);\n    \
                     if (!kept.ok) {{\n        out->error = kept.error;\n        return;\n    }}\n    \
                     out->ok = 1;\n    croft_events_keep({kind}, {key});\n"
                )
            }
            StorageShape::Remove => format!(
                "    croft_string key;\n    {written} kept;\n\n{check}    \
                 croft_clear(out, sizeof *out);\n    croft_storage_key({prefix}, (croft_int) {value}, &key);\n    {storage}_remove(CROFT_VIEW(key), &kept);\n    \
                 if (!kept.ok) {{\n        out->error = kept.error;\n        return;\n    }}\n    out->ok = 1;\n"
            ),
        }
    }

    /// The start of each key: the repository's name in lower case, short
    /// enough to leave room for the largest identity.
    fn key_prefix(&self, repository: &Resolved, aggregate: &Resolved) -> String {
        let digits = self.identity_digits(aggregate);
        let room = KEY_LENGTH.saturating_sub(digits).max(1);
        repository.name.to_lowercase().chars().take(room).collect()
    }

    /// How many digits the largest identity has, from its `Range`, or five
    /// when it has none.
    fn identity_digits(&self, aggregate: &Resolved) -> usize {
        let Type::Named(identity) = self.model.identity_of(aggregate) else {
            return 5;
        };

        let context = &self.program.contexts[&identity.context];

        let declared = context
            .identities
            .iter()
            .find(|i| i.name.name == identity.name);

        let range = declared.and_then(|decl| {
            let range = decl.constraints.iter().find(|c| c.name.text() == "Range")?;
            let high = range.arguments.last()?;
            crate::types::constant::evaluate(self.program, &self.model, &identity.context, high)
        });

        range.map_or(5, |high| high.unsigned_abs().to_string().len())
    }
}
