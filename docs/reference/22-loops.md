# Loops

Services and adapters can repeat a block; domain bodies can't, and iterate with list operations instead. `loop` runs until `break`. A counted loop runs once for each value its counter's constraints allow, in order, and `Range` includes both ends. In a service, `set` can change the service's own `let` names. A loop gives back `Unit`, and `?` inside it leaves the whole service.

A `let` list or map that `set` changes must say with a `Length` how many items it can hold, since the program keeps room for that many: without one, `croft check` reports an error. The `let` keeps its constraints whatever `set` gives it; a value known when the program is compiled that breaks one is an error, and so is a counted loop that is known to add more items than the `Length` allows. One only known while the program runs stops it with a message such as `let items must satisfy Length(0, 200)`, unless the target sets `checks = false`.

```croft
// Plays turn after turn until the player quits or a turn fails.
service PlayGame(): Result<Unit, GameError> {
    calls { Keyboard, Screen }
} = {
    let room_id = FIRST_ROOM
    loop {
        if Keyboard.quit_pressed() {
            break
        }
        set room_id = PlayTurn(room_id)?
    }
}

// runs 24 times: rows 0 to 23
loop (let row where Range(0, 23)) {
    Screen.draw_text("", Position { x: 0, y: row * 8 })
}

// room for 200 items, one for each turn of the loop
let squares: List<Int> where Length(0, 200) = []
loop (let n where Range(1, 200)) {
    set squares = squares.add(n * n)
}

// doesn't compile: `seen` is changed with `set`, so it needs a `Length`
let seen: List<Int> = []
set seen = seen.add(1)

// doesn't compile: this loop runs 200 times and adds to `few` each time
let few: List<Int> where Length(0, 100) = []
loop (let n where Range(1, 200)) {
    set few = few.add(n)
}
```
