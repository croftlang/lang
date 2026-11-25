// Std.Storage: where repositories keep their aggregates. Every program with
// a repository has it, and its names need no import. The compiler turns each
// aggregate into bytes and back; an adapter for `Storage` decides where the
// bytes live. b.toml binds one to each repository, `MemoryStorage` when it
// names none.
context Storage {
    publish {
        Storage,
        StorageError,
        StorageFull,
        StorageDamaged,
        StorageUnavailable,
        MemoryStorage,
        FileStorage
    }

    // There is no room left to save the aggregate.
    error StorageFull

    // What was saved can't be read back as this aggregate: it was written by
    // a different version of the program, or the file is damaged.
    error StorageDamaged

    // The machine couldn't read or write the storage, for any other reason.
    error StorageUnavailable

    enum StorageError {
        include StorageFull
        include StorageDamaged
        include StorageUnavailable
    }

    // Bytes kept under a key, such as `rooms12` for room 12. The key fits the
    // eight characters of a FAT file name.
    port Storage {
        // What is kept under the key, or `None` when nothing is.
        fn load(key: String where Length(1, 8)): Result<Option<Bytes>, StorageError>

        // Keeps the bytes under the key, replacing what was there.
        fn save(key: String where Length(1, 8), data: Bytes): Result<Unit, StorageError>

        fn remove(key: String where Length(1, 8)): Result<Unit, StorageError>
    }

    // Kept in memory while the program runs. The compiler writes its C.
    adapter MemoryStorage for Storage = {
        fn load(key) = native b_memory_storage_load(key)
        fn save(key, data) = native b_memory_storage_save(key, data)
        fn remove(key) = native b_memory_storage_remove(key)
    }

    // A file for each key, such as `rooms12.dat`, through whatever adapter
    // the target binds to `Files`. The compiler writes its C.
    adapter FileStorage for Storage = {
        fn load(key) = native b_file_storage_load(key)
        fn save(key, data) = native b_file_storage_save(key, data)
        fn remove(key) = native b_file_storage_remove(key)
    }
}
