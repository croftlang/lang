# Domain

The whole program, declared once, outside every context. Like a function, it has a contract and a body. The contract holds the context map: every context, with arrows from upstream to downstream. A context may only use contexts upstream of it, and the arrows may not go round in a circle. The contract also imports what the body needs and names the ports it calls, with the context they come from. The body runs first. It only sees what contexts publish, so each sets up its own starting data. Any error passed up with ? stops the program with the error's name.

```croft
// game.dn
domain Game {
    // upstream -> downstream: a context only uses contexts upstream of it
    contexts {
        Dungeon -> Scoring
        Dungeon -> Platform
    }

    use { SetUpRooms, PlayTurn, FIRST_ROOM } from Dungeon
    use { SetUpScoring } from Scoring

    calls { Keyboard as keyboard } from Dungeon
} = {
    SetUpRooms()?
    SetUpScoring()?
    loop {
        let pressed = keyboard.pressed()
        if pressed == Some(Quit) {
            break
        }
        PlayTurn(FIRST_ROOM, pressed)?
    }
}

// in context Dungeon, published for the domain
service SetUpRooms(): Result<Unit, GameError> {
    calls { Rooms }
} = {
    let player = Room.Player { id: 1, position: Position { x: 16, y: 16 }, lives: 3 }
    let room = Room.Open(id: FIRST_ROOM, player: player, exit: Position { x: 232, y: 168 })?
    Rooms.save(room)?
}
```
