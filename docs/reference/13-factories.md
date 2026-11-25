# Factories

The only way to make a new aggregate. A factory is declared inside it, takes named arguments, can have a contract and has a body that gives back the new aggregate. It can raise events with emit. The aggregate is held to its invariants before it exists: if one breaks, the factory fails with its error and its events are dropped. An aggregate written out with its fields is only allowed inside its own factories, or in an adapter written for its repository, which builds it from what it kept. Services, policies and functions call a factory by the aggregate's name and its own, and get back a Result with the aggregate's errors. Only the context that owns the aggregate calls its factories.

```croft
aggregate Room {
    id: RoomId
    player: Player
    enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive
    exit: Position

    invariant ExitClear or ExitBlocked

    error ExitBlocked
    event Opened { exit: Position }

    // A new room starts with no enemies; its exit must be clear.
    factory Open(id: RoomId, player: Player, exit: Position) = {
        emit Opened { exit }
        Room { id, player, enemies: [], exit }
    }
}

// in a service
let room = Room.Open(id: FIRST_ROOM, player: player, exit: Position { x: 232, y: 168 })?
rooms.save(room)?

// anywhere else, an aggregate written out is an error
let room = Room { ... }        // a new `Room` is made by its factory: `Room.Open`
rooms.save(room with { ... })  // a `Room` changes by issuing one of its commands
```
