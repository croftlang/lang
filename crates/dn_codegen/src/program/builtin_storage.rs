//! The C for the built-in adapters for `Storage`, when a repository uses
//! them: `MemoryStorage`, a table of keys and bytes, and `FileStorage`, a
//! file for each key through the target's `Files` adapter.

use dn_analysis::Type;

use crate::names::{c_name, error_code};
use crate::Generator;

/// How many aggregates `MemoryStorage` holds for each repository using it.
const MEMORY_SLOTS: usize = 8;

impl<'a> Generator<'a> {
    pub fn builtin_storage(&self) -> String {
        let uses = |name: &str| {
            (self.storages.values())
                .filter(|s| s.context == "Std.Storage" && s.name == name)
                .count()
        };
        let mut out = String::new();
        let memory = uses("MemoryStorage");
        if memory > 0 {
            out.push_str(&self.memory_storage(memory * MEMORY_SLOTS));
        }
        if uses("FileStorage") > 0 {
            out.push_str(&self.file_storage());
        }
        out
    }

    fn storage_types(&self) -> (String, String) {
        let loaded = self.c_type(&Type::result(
            Type::option(Type::Bytes),
            Type::Errors(Vec::new()),
        ));
        let written = self.c_type(&Type::result(Type::Unit, Type::Errors(Vec::new())));
        (loaded, written)
    }

    fn storage_error(&self, name: &str) -> String {
        let context = if name.starts_with("Storage") {
            "Std.Storage"
        } else {
            "Std.Files"
        };
        self.model
            .lookup(context, name)
            .map_or("0".to_string(), |error| error_code(&error))
    }

    fn memory_storage(&self, slots: usize) -> String {
        let (loaded, written) = self.storage_types();
        let full = self.storage_error("StorageFull");
        format!(
            "static dn_string dn_memory_keys[{slots}];\nstatic dn_bytes dn_memory_values[{slots}];\nstatic dn_u16 dn_memory_count = 0;\n\n\
             /* Where a key is kept, or dn_memory_count when it isn't. */\nstatic dn_u16 dn_memory_find(const dn_string *key)\n{{\n    dn_u16 index;\n\n    \
             for (index = 0; index < dn_memory_count; index++) {{\n        if (dn_string_equal(&dn_memory_keys[index], key)) {{\n            return index;\n        }}\n    }}\n    return dn_memory_count;\n}}\n\n\
             void dn_memory_storage_load(const dn_string *key, {loaded} *out)\n{{\n    dn_u16 index = dn_memory_find(key);\n\n    \
             dn_clear(out, sizeof *out);\n    out->ok = 1;\n    if (index < dn_memory_count) {{\n        out->value.some = 1;\n        out->value.value = dn_memory_values[index];\n    }}\n}}\n\n\
             void dn_memory_storage_save(const dn_string *key, const dn_bytes *data, {written} *out)\n{{\n    dn_u16 index = dn_memory_find(key);\n\n    \
             dn_clear(out, sizeof *out);\n    if (index == dn_memory_count) {{\n        if (dn_memory_count == {slots}) {{\n            out->error.code = {full};\n            return;\n        }}\n        \
             dn_memory_keys[index] = *key;\n        dn_memory_count++;\n    }}\n    dn_memory_values[index] = *data;\n    out->ok = 1;\n}}\n\n\
             void dn_memory_storage_remove(const dn_string *key, {written} *out)\n{{\n    dn_u16 index = dn_memory_find(key);\n\n    \
             dn_clear(out, sizeof *out);\n    out->ok = 1;\n    if (index < dn_memory_count) {{\n        dn_memory_count--;\n        \
             dn_memory_keys[index] = dn_memory_keys[dn_memory_count];\n        dn_memory_values[index] = dn_memory_values[dn_memory_count];\n    }}\n}}\n\n"
        )
    }

    fn file_storage(&self) -> String {
        let (loaded, written) = self.storage_types();
        let read = self.c_type(&Type::result(Type::Bytes, Type::Errors(Vec::new())));
        let files = (self.bindings.iter())
            .find(|(port, _)| port.context == "Std.Files" && port.name == "Files")
            .map_or("dn_no_files".to_string(), |(_, adapter)| c_name(adapter));
        let missing = self.storage_error("NotFound");
        let no_room = self.storage_error("NoRoom");
        let full = self.storage_error("StorageFull");
        let unavailable = self.storage_error("StorageUnavailable");
        format!(
            "/* The file for a key: the key and \".dat\". */\nstatic void dn_file_path(const dn_string *key, dn_string *out)\n{{\n    dn_string extension;\n\n    \
             dn_string_set(&extension, \".dat\");\n    dn_string_join(key, &extension, out);\n}}\n\n\
             void dn_file_storage_load(const dn_string *key, {loaded} *out)\n{{\n    dn_string path;\n    {read} file;\n\n    \
             dn_clear(out, sizeof *out);\n    dn_file_path(key, &path);\n    {files}_read(&path, &file);\n    \
             if (file.ok) {{\n        out->ok = 1;\n        out->value.some = 1;\n        out->value.value = file.value;\n    }} else if (file.error.code == {missing}) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = {unavailable};\n    }}\n}}\n\n\
             void dn_file_storage_save(const dn_string *key, const dn_bytes *data, {written} *out)\n{{\n    dn_string path;\n    {written} file;\n\n    \
             dn_clear(out, sizeof *out);\n    dn_file_path(key, &path);\n    {files}_write(&path, data, &file);\n    \
             if (file.ok) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = file.error.code == {no_room} ? {full} : {unavailable};\n    }}\n}}\n\n\
             void dn_file_storage_remove(const dn_string *key, {written} *out)\n{{\n    dn_string path;\n    {written} file;\n\n    \
             dn_clear(out, sizeof *out);\n    dn_file_path(key, &path);\n    {files}_remove(&path, &file);\n    \
             if (file.ok || file.error.code == {missing}) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = {unavailable};\n    }}\n}}\n\n"
        )
    }
}
