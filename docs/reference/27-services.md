# Services

The outer layer, where the program meets the world. A service lists the ports it calls, loads aggregates, issues commands and calls ports.

```croft
// Applies one key press to the room, saves it and redraws it.
service PlayTurn(room_id: RoomId, pressed: Option<Key>): Result<Unit, GameError> {
    calls Screen, Rooms
} = {
    let room = rooms.find(room_id)?
    let moved = match pressed {
        Some(key) -> issue Move(direction: direction_for(key)) on room?
        None -> room
    }
    rooms.save(moved)?
    screen.draw_sprite(PLAYER_SPRITE, moved.player.position)
}
```
