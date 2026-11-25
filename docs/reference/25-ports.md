# Ports

What the domain needs from the outside world, by signature only: never how it is done. An operation with no return type gives back Unit. `Arguments` and `Console` are built in: every target supplies them, so dn.toml never names an adapter for them.

```croft
port Screen {
    fn clear()
    fn draw_sprite(sprite: SpriteId, at: Position)
    fn draw_text(text: String where Length(0, 32), at: Position)
}

port Keyboard {
    fn pressed(): Option<Key>
}

// built in: the command-line arguments, empty on machines without one
service Start(): Bool {
    calls Arguments
} = arguments.all().contains("--demo")

// built in: prints a line
service Greet() {
    calls Console
} = console.print("Welcome to the dungeon")
```
