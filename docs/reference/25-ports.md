# Ports

What the domain needs from the outside world, by signature only: never how it is done. An operation with no return type gives back Unit. `Console`, `Arguments`, `Environment` and `Process` are declared in Std.Console, which every program has without an import: every target supplies them, so dn.toml never names an adapter for them. A port of the program's own with the same name replaces one. A service lists the ports it calls in `calls`, and calls each by that name, or by the alias after `as`. An operation can have a contract, with constraints on its parameters or lines such as `low <= high`, which every caller keeps whatever adapter fills the port: broken by known arguments it is an error from `croft check`, and otherwise the program stops when the adapter's operation starts, unless the target sets `checks = false`. A port has nowhere to fail, so its contract lines can't end with `or` and an error; an operation that can fail gives back a `Result`.

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

// built in: an environment variable, None when it isn't set
service Home(): String {
    calls { Environment }
} = Environment.get("HOME").unwrap_or("/")

// built in: prints a line
service Greet() {
    calls { Console }
} = Console.print("Welcome to the dungeon")

// built in: reads a line, None at the end of the input, and prints a line
// on the error output
service Echo() {
    calls { Console }
} = match Console.read_line() {
    Some(line) -> Console.print(line)
    None -> Console.print_error("nothing to read")
}

// built in: stops the program with an exit code from 0 to 255
service Quit() {
    calls { Process }
} = Process.exit(2)
```

`Console.read_line()` gives the next line without its `"\n"` or `"\r\n"`, and None at the end of the input; a target with nothing to read gives None. A line is never cut short with checks on: one longer than a string stops the program, as an argument longer than a string does. With `checks = false`, as much of the line as a string holds is kept and the rest of that line is skipped. `Console.print_error` writes to the error output, kept apart from what `Console.print` writes. `Environment.get(name)` gives the value of the environment variable with that name, through C's `getenv`, and None when it isn't set; a target without environment variables gives None, and so does a name no variable can have: an empty one, one longer than 127 characters, or one holding `=` or a NUL byte. A value is never cut short with checks on: one longer than a string stops the program, as an argument does, and with `checks = false` as much of it as a string holds is kept. `Process.exit` stops the program at once with its code; a code known to be outside 0 to 255 is an error from `croft check`. One found only while the program runs stops it with checks on, and keeps its lowest 8 bits with `checks = false`.
