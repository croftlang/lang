# Handling failure

issue and find return a Result. Pass a failure up with ?, which works when the body's error type includes the aggregate's errors, or replace it with a specific error using or. An unhandled Result doesn't compile.

```croft
enum GameError {
    include Room.Error
    include Scoreboard.Error
    include StorageError
    KeyboardUnavailable
}

// pass the aggregate's own error up
let room = rooms.find(room_id)?
issue Attack(enemy_id: target, damage: 5) on Room(room_id)?

// fail with a more specific error instead
issue Reset on Scoreboard(MAIN_SCOREBOARD) or GameError.ScoreboardUnavailable
```
