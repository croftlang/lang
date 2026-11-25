# Repositories

A port for loading and saving one aggregate. The compiler writes find and save, turning each aggregate into bytes and back, and each target binds the repository in dn.toml to an adapter for the built-in Storage port, which keeps the bytes under a key: MemoryStorage, the default, FileStorage, or one of the program's own. An adapter can also be written for the repository itself, to keep aggregates its own way, such as in table columns. StorageError and Storage are built in, and need no import.

```croft
repository Rooms for Room
// gives: find(id: RoomId): Result<Room, Room.Error or StorageError>
//        save(room: Room): Result<Unit, StorageError>

// StorageError: StorageFull, StorageDamaged, StorageUnavailable
// StorageDamaged: written by a different version of the program, or damaged

// built in: bytes under a key such as rooms12
port Storage {
    fn load(key: String where Length(1, 8)): Result<Option<Bytes>, StorageError>
    fn save(key: String where Length(1, 8), data: Bytes): Result<Unit, StorageError>
    fn remove(key: String where Length(1, 8)): Result<Unit, StorageError>
}

// where the bytes live
adapter RedisStorage for Storage = {
    fn load(key) = native redis_get(key)
    fn save(key, data) = native redis_set(key, data)
    fn remove(key) = native redis_del(key)
}

// or how aggregates are kept, written for one repository
adapter PostgresRooms for Rooms = {
    fn find(id) = native pg_find_room(id)
    fn save(room) = native pg_save_room(room)
}

// dn.toml
// [target.spectrum_48k.adapters]
// Files = "DivmmcFiles"
// Rooms = "FileStorage"      // rooms12.dat for room 12
//
// [target.server.adapters]
// Rooms = "PostgresRooms"
```
