//! `FileStorage`: each aggregate saved in a file of its own, named after its
//! repository and identity, through whatever adapter the target binds to
//! `Files`. A file starts with the hash of the aggregate's layout, so one
//! written by a different version of the program reads as `StorageDamaged`.

use dn_analysis::types::Resolved;
use dn_analysis::Type;

use super::layout::layout_hash;
use crate::names::{aggregate_kind, c_name, c_string, error_code};
use crate::Generator;

/// FAT disks, such as a DivMMC's, allow eight characters before `.dat`.
const NAME_LENGTH: usize = 8;

impl<'a> Generator<'a> {
    pub(super) fn file_repository(
        &self,
        repository: &Resolved,
        aggregate: &Resolved,
        find: &str,
        save: &str,
    ) -> String {
        let aggregate_type = Type::Named(aggregate.clone());
        let serializer = self.serializer_for(&aggregate_type);
        let hash = layout_hash(&self.layout(&aggregate_type));
        let field = self.identity_field(aggregate);
        let prefix = c_string(&self.file_prefix(repository, aggregate));
        let files = self.files_adapter();
        let read_result = self.c_type(&Type::result(Type::Bytes, Type::Errors(Vec::new())));
        let write_result = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));
        let error = |context: &str, name: &str| {
            self.model
                .lookup(context, name)
                .map_or("0".to_string(), |error| error_code(&error))
        };
        let not_found = error(&aggregate.context, &format!("{}.NotFound", aggregate.name));
        let file_missing = error("Std.Files", "NotFound");
        let file_full = error("Std.Files", "NoRoom");
        let full = error("Std.Storage", "StorageFull");
        let damaged = error("Std.Storage", "StorageDamaged");
        let unavailable = error("Std.Storage", "StorageUnavailable");
        let kind = aggregate_kind(aggregate);
        let key = self.event_key(aggregate, &format!("aggregate->{field}"));
        format!(
            "{find}\n{{\n    dn_string path;\n    {read_result} file;\n    dn_u16 at = 0;\n    dn_int layout = 0;\n\n    \
             dn_clear(out, sizeof *out);\n    dn_storage_path({prefix}, (dn_int) identity, &path);\n    {files}_read(&path, &file);\n    \
             if (!file.ok) {{\n        out->error.code = file.error.code == {file_missing} ? {not_found} : {unavailable};\n        return;\n    }}\n    \
             if (!dn_bytes_take_int(&file.value, &at, &layout) || layout != {hash}L\n        \
             || !dn_read_{serializer}(&file.value, &at, &out->value) || at != file.value.length) {{\n        \
             dn_clear(out, sizeof *out);\n        out->error.code = {damaged};\n        return;\n    }}\n    out->ok = 1;\n}}\n\n\
             {save}\n{{\n    dn_string path;\n    dn_bytes data;\n    {write_result} written;\n\n    \
             dn_clear(out, sizeof *out);\n    data.length = 0;\n    \
             if (!dn_bytes_put_int(&data, {hash}L) || !dn_write_{serializer}(aggregate, &data)) {{\n        \
             out->error.code = {full};\n        return;\n    }}\n    \
             dn_storage_path({prefix}, (dn_int) aggregate->{field}, &path);\n    {files}_write(&path, &data, &written);\n    \
             if (!written.ok) {{\n        out->error.code = written.error.code == {file_full} ? {full} : {unavailable};\n        return;\n    }}\n    \
             out->ok = 1;\n    dn_events_keep({kind}, {key});\n}}\n\n"
        )
    }

    /// The C name of the adapter the target binds to `Files`.
    fn files_adapter(&self) -> String {
        let adapter = self
            .bindings
            .iter()
            .find(|(port, _)| port.context == "Std.Files" && port.name == "Files")
            .map(|(_, adapter)| adapter);
        adapter.map_or("dn_no_files".to_string(), c_name)
    }

    /// The start of each file's name: the repository's name in lower case,
    /// short enough to leave room for the largest identity.
    fn file_prefix(&self, repository: &Resolved, aggregate: &Resolved) -> String {
        let digits = self.identity_digits(aggregate);
        let room = NAME_LENGTH.saturating_sub(digits).max(1);
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
