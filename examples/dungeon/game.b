// The game: the contexts it is made of, and what runs first. It sets up
// each context's starting data, then plays turn after turn until the player
// quits or a turn fails. `--title` shows the title first.
domain Game {
    // Scoring reacts to Dungeon's events; Platform fills Dungeon's ports.
    contexts {
        Dungeon -> Scoring
        Dungeon -> Platform
    }

    use { SetUpRooms, PlayTurn, FIRST_ROOM, GAME_TITLE, Position, Screen, Keyboard } from Dungeon
    use { SetUpScoring } from Scoring

    calls Screen, Keyboard, Arguments
} = {
    SetUpRooms()?
    SetUpScoring()?
    screen.clear()
    if arguments.all().contains("--title") {
        screen.draw_text(GAME_TITLE, Position { x: 0, y: 0 })
    }
    loop {
        let pressed = keyboard.pressed()
        if pressed == Some(Quit) {
            break
        }
        PlayTurn(FIRST_ROOM, pressed)?
    }
}
