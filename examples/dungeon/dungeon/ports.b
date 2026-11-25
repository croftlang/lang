// What the dungeon needs from the machine it runs on.
context Dungeon

port Screen {
    fn clear(): Unit
    fn draw_sprite(sprite: SpriteId, at: Position): Unit
    fn draw_text(text: String where Length(0, 32), at: Position): Unit
}

port Keyboard {
    fn pressed(): Option<Key>
}

repository Rooms for Room
