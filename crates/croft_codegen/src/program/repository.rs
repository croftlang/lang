//! Repositories. Each aggregate is turned into bytes and kept under a key,
//! such as `rooms12` for room 12, by the adapter for `Storage` that croft.toml
//! binds to the repository: `MemoryStorage` unless it names another. A
//! saved value starts with the hash of the aggregate's layout, so one written
//! by a different version of the program loads as `StorageDamaged`. Saving
//! keeps the events the aggregate's commands raised, so policies hear of them.

use croft_analysis::symbols::SymbolKind;
use croft_analysis::types::Resolved;
use croft_analysis::Type;

use super::layout::layout_hash;
use crate::names::{aggregate_kind, c_name, c_string, error_code};
use crate::Generator;

/// Keys fit the eight characters of a FAT file name.
const KEY_LENGTH: usize = 8;

impl<'a> Generator<'a> {
    pub fn repositories(&self, prototypes: &mut Vec<String>) -> String {
        let mut out = String::new();
        for repository in self.model.all(SymbolKind::Repository) {
            let Some(aggregate) = self.model.repository_aggregate(&repository) else {
                continue;
            };
            let Some(storage) = self.storages.get(&repository) else {
                continue;
            };
            let name = c_name(&repository);
            let aggregate_type = Type::Named(aggregate.clone());
            let item = self.c_type(&aggregate_type);
            let identity_type = self.c_type(&self.model.identity_of(&aggregate));
            let found = self.c_type(&Type::result(aggregate_type, Type::Errors(Vec::new())));
            let saved = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));
            let find = format!("void {name}_find({identity_type} identity, {found} *out)");
            let save = format!("void {name}_save(const {item} *aggregate, {saved} *out)");
            prototypes.push(format!("{find};"));
            prototypes.push(format!("{save};"));
            if self.is_repository_adapter(&repository, storage) {
                out.push_str(&self.handwritten_repository(&aggregate, storage, &find, &save));
                continue;
            }
            out.push_str(&self.repository_functions(
                &repository,
                &aggregate,
                storage,
                &find,
                &save,
            ));
        }
        out
    }

    fn repository_functions(
        &self,
        repository: &Resolved,
        aggregate: &Resolved,
        storage: &Resolved,
        find: &str,
        save: &str,
    ) -> String {
        let aggregate_type = Type::Named(aggregate.clone());
        let serializer = self.serializer_for(&aggregate_type);
        let hash = layout_hash(&self.layout(&aggregate_type));
        let field = self.identity_field(aggregate);
        let prefix = c_string(&self.key_prefix(repository, aggregate));
        let storage = c_name(storage);
        let loaded = self.c_type(&Type::result(
            Type::option(Type::Bytes),
            Type::Errors(Vec::new()),
        ));
        let written = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));
        let not_found = self
            .model
            .lookup(&aggregate.context, &format!("{}.NotFound", aggregate.name))
            .map_or("0".to_string(), |error| error_code(&error));
        let error = |name: &str| {
            self.model
                .lookup("Std.Storage", name)
                .map_or("0".to_string(), |error| error_code(&error))
        };
        let (full, damaged) = (error("StorageFull"), error("StorageDamaged"));
        let kind = aggregate_kind(aggregate);
        let key = self.event_key(aggregate, &format!("aggregate->{field}"));
        format!(
            "{find}\n{{\n    croft_string key;\n    {loaded} kept;\n    croft_u16 at = 0;\n    croft_int layout = 0;\n\n    \
             croft_clear(out, sizeof *out);\n    croft_storage_key({prefix}, (croft_int) identity, &key);\n    {storage}_load(&key, &kept);\n    \
             if (!kept.ok) {{\n        out->error = kept.error;\n        return;\n    }}\n    \
             if (!kept.value.some) {{\n        out->error.code = {not_found};\n        return;\n    }}\n    \
             if (!croft_bytes_take_int(&kept.value.value, &at, &layout) || layout != {hash}L\n        \
             || !croft_read_{serializer}(&kept.value.value, &at, &out->value) || at != kept.value.value.length) {{\n        \
             croft_clear(out, sizeof *out);\n        out->error.code = {damaged};\n        return;\n    }}\n    out->ok = 1;\n}}\n\n\
             {save}\n{{\n    croft_string key;\n    croft_bytes data;\n    {written} kept;\n\n    \
             croft_clear(out, sizeof *out);\n    data.length = 0;\n    \
             if (!croft_bytes_put_int(&data, {hash}L) || !croft_write_{serializer}(aggregate, &data)) {{\n        \
             out->error.code = {full};\n        return;\n    }}\n    \
             croft_storage_key({prefix}, (croft_int) aggregate->{field}, &key);\n    {storage}_save(&key, &data, &kept);\n    \
             if (!kept.ok) {{\n        out->error = kept.error;\n        return;\n    }}\n    \
             out->ok = 1;\n    croft_events_keep({kind}, {key});\n}}\n\n"
        )
    }

    /// A repository whose adapter writes `find` and `save` by hand: they are
    /// called as they are, and saving still keeps the aggregate's events.
    fn handwritten_repository(
        &self,
        aggregate: &Resolved,
        adapter: &Resolved,
        find: &str,
        save: &str,
    ) -> String {
        let adapter = c_name(adapter);
        let field = self.identity_field(aggregate);
        let kind = aggregate_kind(aggregate);
        let key = self.event_key(aggregate, &format!("aggregate->{field}"));
        format!(
            "{find}\n{{\n    {adapter}_find(identity, out);\n}}\n\n\
             {save}\n{{\n    {adapter}_save(aggregate, out);\n    if (out->ok) {{\n        croft_events_keep({kind}, {key});\n    }}\n}}\n\n"
        )
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
