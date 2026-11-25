# Loops

Services and adapters can repeat a block; domain bodies can't, and iterate with list operations instead. `loop` runs until `break`. A counted loop runs once for each value its counter's constraints allow, in order, and `Range` includes both ends. In a service, `set` can change the service's own `let` names. A loop gives back `Unit`, and `?` inside it leaves the whole service.

```croft
// Plays turn after turn until the player quits or a turn fails.
service PlayGame(): Result<Unit, GameError> {
    calls Keyboard, Screen
} = {
    let room_id = FIRST_ROOM
    loop {
        if keyboard.quit_pressed() {
            break
        }
        set room_id = PlayTurn(room_id)?
    }
}

// runs 24 times: rows 0 to 23
loop (let row where Range(0, 23)) {
    screen.draw_text("", Position { x: 0, y: row * 8 })
}
```
