//! The C for the built-in adapters for `Storage`, when a repository uses
//! them: `MemoryStorage`, a table of keys and bytes, and `FileStorage`, a
//! file for each key through the target's `Files` adapter.

use croft_analysis::Type;

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
            "static croft_string croft_memory_keys[{slots}];\nstatic croft_bytes croft_memory_values[{slots}];\nstatic croft_u16 croft_memory_count = 0;\n\n\
             /* Where a key is kept, or croft_memory_count when it isn't. */\nstatic croft_u16 croft_memory_find(const croft_string *key)\n{{\n    croft_u16 index;\n\n    \
             for (index = 0; index < croft_memory_count; index++) {{\n        if (croft_string_equal(&croft_memory_keys[index], key)) {{\n            return index;\n        }}\n    }}\n    return croft_memory_count;\n}}\n\n\
             void croft_memory_storage_load(const croft_string *key, {loaded} *out)\n{{\n    croft_u16 index = croft_memory_find(key);\n\n    \
             croft_clear(out, sizeof *out);\n    out->ok = 1;\n    if (index < croft_memory_count) {{\n        out->value.some = 1;\n        out->value.value = croft_memory_values[index];\n    }}\n}}\n\n\
             void croft_memory_storage_save(const croft_string *key, const croft_bytes *data, {written} *out)\n{{\n    croft_u16 index = croft_memory_find(key);\n\n    \
             croft_clear(out, sizeof *out);\n    if (index == croft_memory_count) {{\n        if (croft_memory_count == {slots}) {{\n            out->error.code = {full};\n            return;\n        }}\n        \
             croft_memory_keys[index] = *key;\n        croft_memory_count++;\n    }}\n    croft_memory_values[index] = *data;\n    out->ok = 1;\n}}\n\n\
             void croft_memory_storage_remove(const croft_string *key, {written} *out)\n{{\n    croft_u16 index = croft_memory_find(key);\n\n    \
             croft_clear(out, sizeof *out);\n    out->ok = 1;\n    if (index < croft_memory_count) {{\n        croft_memory_count--;\n        \
             croft_memory_keys[index] = croft_memory_keys[croft_memory_count];\n        croft_memory_values[index] = croft_memory_values[croft_memory_count];\n    }}\n}}\n\n"
        )
    }

    fn file_storage(&self) -> String {
        let (loaded, written) = self.storage_types();
        let read = self.c_type(&Type::result(Type::Bytes, Type::Errors(Vec::new())));
        let files = (self.bindings.iter())
            .find(|(port, _)| port.context == "Std.Files" && port.name == "Files")
            .map_or("croft_no_files".to_string(), |(_, adapter)| c_name(adapter));
        let missing = self.storage_error("NotFound");
        let no_room = self.storage_error("NoRoom");
        let full = self.storage_error("StorageFull");
        let unavailable = self.storage_error("StorageUnavailable");
        format!(
            "/* The file for a key: the key and \".dat\". */\nstatic void croft_file_path(const croft_string *key, croft_string *out)\n{{\n    croft_string extension;\n\n    \
             croft_string_set(&extension, \".dat\");\n    croft_string_join(key, &extension, out);\n}}\n\n\
             void croft_file_storage_load(const croft_string *key, {loaded} *out)\n{{\n    croft_string path;\n    {read} file;\n\n    \
             croft_clear(out, sizeof *out);\n    croft_file_path(key, &path);\n    {files}_read(&path, &file);\n    \
             if (file.ok) {{\n        out->ok = 1;\n        out->value.some = 1;\n        out->value.value = file.value;\n    }} else if (file.error.code == {missing}) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = {unavailable};\n    }}\n}}\n\n\
             void croft_file_storage_save(const croft_string *key, const croft_bytes *data, {written} *out)\n{{\n    croft_string path;\n    {written} file;\n\n    \
             croft_clear(out, sizeof *out);\n    croft_file_path(key, &path);\n    {files}_write(&path, data, &file);\n    \
             if (file.ok) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = file.error.code == {no_room} ? {full} : {unavailable};\n    }}\n}}\n\n\
             void croft_file_storage_remove(const croft_string *key, {written} *out)\n{{\n    croft_string path;\n    {written} file;\n\n    \
             croft_clear(out, sizeof *out);\n    croft_file_path(key, &path);\n    {files}_remove(&path, &file);\n    \
             if (file.ok || file.error.code == {missing}) {{\n        out->ok = 1;\n    }} else {{\n        out->error.code = {unavailable};\n    }}\n}}\n\n"
        )
    }
}
