# Aggregates and entities

An aggregate is the consistency boundary around its entities and values. Its own fields form the root. Entities live only inside the aggregate that owns them; outside it they are known by their identity. Something with identity that stands on its own is an aggregate with no entities. A new aggregate is only made by one of its [factories](13-factories.md). An aggregate also declares its own errors, which are part of the ubiquitous language. Each invariant names the error it fails with; if a command would break one, its changes are thrown away and the aggregate is left as it was.

```croft
aggregate Room {
    id: RoomId
    player: Player
    enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive
    exit: Position

    entity Player {
        id: PlayerId
        position: Position
        lives: Int where Range(0, 9)
    }

    entity Enemy {
        id: EnemyId
        position: Position
        hit_points: Int where Range(0, 100)
    }

    invariant ExitClear or ExitBlocked
    invariant EnemiesApart or EnemiesOverlap

    error NotFound
    error EnemyMissing
    error ExitBlocked
    error EnemiesOverlap
}
```
