# Contexts

Everything is declared inside a context, a bounded context with its own
meanings. A context publishes the terms other contexts may use, and imports only
what it needs with `use { ... } from`, and can give a term an alias with
`as` when another name reads better. A context can be written in braces, or
opened with a header line at the top of a file, which claims the rest of the
file. An `aggregate` header does the same for one aggregate. The compiler joins
every part of a context, and of an aggregate, before checking them. A comment
directly above `context` documents it. When a context is written in several
files, the comment above the part with `publish` is the context's doc; the
others describe their own file.

```croft
// Rooms, the enemies in them, and moving between them.
context Dungeon {
    publish {
        RoomId,
        EnemyId,
        Position,
        Room.EnemyDefeated
    }
}

context Scoring {
    use {
        EnemyId,
        Room.EnemyDefeated as EnemyDefeated
    } from Dungeon
}

// dungeon/room/commands.dn: a header claims the rest of the file
context Dungeon
aggregate Room

command Move(direction: Direction) = {
    ...
}
```
