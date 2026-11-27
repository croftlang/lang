# Repositories

A port for keeping one aggregate. The developer declares its operations, named in the ubiquitous language, and every operation works on the aggregate it is for: each one takes or gives back that aggregate, its identity, or plain values, and never another aggregate. Every operation can fail with StorageError, so swapping one adapter for another never changes a signature. An adapter for the repository keeps aggregates its own way, such as in table columns. Or croft.toml can bind the repository to an adapter for the built-in Storage port, which keeps the bytes the compiler makes from each aggregate under a key: MemoryStorage, the default, FileStorage, or one of the program's own. Storage can only provide operations of three shapes: find by identity, giving back an Option or failing with the aggregate's NotFound error; save the aggregate; and remove by identity. A repository with any other operation needs an adapter of its own, and check says so for each target that binds it to Storage. Issuing a command on `Room(id)` loads and saves through the repository, so it needs a find that fails with NotFound, and a save. Written with no body, a repository has find and save. An operation's parameters can have constraints, as a port's can, such as `fn keep(bed: Bed where Made)`; one broken while the program runs stops it with a message naming the operation, `Beds.keep: bed must satisfy Made`, unless the target sets `checks = false`. StorageError and Storage are built in, and need no import.

```croft
// no body: find and save
repository Rooms for Room
// gives: find(id: RoomId): Result<Room, Room.Error or StorageError>
//        save(room: Room): Result<Unit, StorageError>

// its own operations
repository Cargos for Cargo {
    fn find(id: CargoId): Result<Option<Cargo>, StorageError>
    fn awaiting_customs(): Result<List<Cargo>, StorageError>
    fn save(cargo: Cargo): Result<Unit, StorageError>
    fn remove(id: CargoId): Result<Unit, StorageError>
}

// a find that fails when nothing is kept, as issuing on Cargo(id) needs
enum Lookup {
    include Cargo.NotFound
    include StorageError
}
// fn load(id: CargoId): Result<Cargo, Lookup>

// StorageError: StorageFull, StorageDamaged, StorageUnavailable
// StorageDamaged: written by a different version of the program, or damaged:
// a number that doesn't fit its field, a variant or flag that isn't one, a
// map key or set item kept twice, or a broken constraint or invariant

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

// how cargos are kept, written for the repository
adapter PostgresCargos for Cargos = {
    fn find(id) = native pg_find_cargo(id)
    fn awaiting_customs() = native pg_cargos_awaiting_customs()
    fn save(cargo) = native pg_save_cargo(cargo)
    fn remove(id) = native pg_remove_cargo(id)
}

// croft.toml
// [target.spectrum_48k.adapters]
// Files = "DivmmcFiles"
// Rooms = "FileStorage"      // rooms12.dat for room 12
//
// [target.server.adapters]
// Rooms = "MemoryStorage"
// Cargos = "PostgresCargos"  // Storage has no awaiting_customs, so Cargos needs its own adapter
```

Storage keeps bytes from outside the program, which can be damaged or changed. Finding an aggregate checks everything it reads back: each number fits its field and its `Range`, each enum tag names a variant, each `Bool` and option is true or false, no map key or set item is kept twice, and every field constraint and invariant holds. Anything else gives back `StorageDamaged`, on every target, whatever its `checks` setting.
