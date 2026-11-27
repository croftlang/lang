# Ports

What the domain needs from the outside world, by signature only: never how it is done. An operation with no return type gives back Unit. `Arguments` and `Console` are declared in Std.Console, which every program has without an import: every target supplies them, so dn.toml never names an adapter for them. A port of the program's own with the same name replaces one. A service lists the ports it calls in `calls`, and calls each by that name, or by the alias after `as`. An operation can have a contract, with constraints on its parameters or lines such as `low <= high`, which every caller keeps whatever adapter fills the port: broken by known arguments it is an error from `croft check`, and otherwise the program stops when the adapter's operation starts, unless the target sets `checks = false`. A port has nowhere to fail, so its contract lines can't end with `or` and an error; an operation that can fail gives back a `Result`.

```croft
port Screen {
    fn clear()
    fn draw_sprite(sprite: SpriteId, at: Position)
    fn draw_text(text: String where Length(0, 32), at: Position)
}

port Keyboard {
    fn pressed(): Option<Key>
}

// a contract every caller keeps: `Dice.roll(6, 1)` doesn't compile
port Dice {
    fn roll(low: Int, high: Int): Int {
        low <= high
    }
}

// built in: the command-line arguments, empty on machines without one
service Start(): Bool {
    calls { Arguments }
} = Arguments.all().contains("--demo")

// built in: prints a line
service Greet() {
    calls { Console }
} = Console.print("Welcome to the dungeon")
```
