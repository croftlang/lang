# dn

dn is a domain language. The domain model is the program: you write it with
domain-driven design's building blocks (contexts, values, entities,
aggregates, commands, events, ports and adapters) and the compiler checks the
rules you attach to them. dn is designed to compile to portable C, so one
program can run on a ZX Spectrum, an Atari ST, Windows, macOS and Linux.

```dn
context Dungeon {
    aggregate Room {
        identity id: RoomId
        enemies: List<Enemy> where Length(0, MAX_ENEMIES), each Alive
        exit: Position

        invariant ExitClear or ExitBlocked

        error NotFound
        error EnemyMissing { enemy_id: EnemyId }

        // Damages one enemy, and defeats it at zero hit points.
        command Attack(enemy_id: EnemyId, damage: Int where Range(1, 20)) {
            enemies.contains(enemy_id) or EnemyMissing { enemy_id }
        } = {
            let enemy = enemies.get(enemy_id)
            let remaining = max(0, enemy.hit_points - damage)
            set enemies = enemies.replace(enemy with { hit_points: remaining })
            emit EnemyDamaged { enemy_id, remaining }
        }
    }
}
```

## Splitting a context across files

A context can be written inline, in braces, or opened with a header line at
the top of a file. A header claims the rest of the file. Add an `aggregate`
header to write part of one aggregate:

```dn
// dungeon/room/commands.dn
context Dungeon
aggregate Room

command Move(direction: Direction) = {
    ...
}
```

The compiler joins every file that opens the same context, and every part of
the same aggregate, before checking. This keeps each file short.

## Status

The design is still being decided. The terms are in
[docs/glossary.md](docs/glossary.md).

The compiler reads the current syntax and checks declarations: every name
resolves, rules and invariants name real constraints and errors, aggregates
and entities have one identity, adapters provide their port's operations, and
`dn.toml` gives every port its services use an adapter on each target.

It also type-checks bodies. Every expression gets a type, failures must be
handled with `?` or `or`, `match` must handle every variant, and each kind of
body can only do what its kind allows: only commands `set` fields and `emit`
events, only services and adapters loop, and only adapters call native code.

`dn compile` writes portable C89 for one target. It has been tested with gcc
and clang in strict C89 mode; building with cc65 and z88dk is next.

## Usage

```sh
cargo install --path crates/dn_cli

dn check .                       # check every .dn file in the folder and its subfolders
dn compile . --target desktop    # check, then write C to target/dn/desktop/
```

Try it on the example, a small dungeon game split across twelve files:

```sh
dn check examples/dungeon
dn compile examples/dungeon --target spectrum_48k
```

## The project file

`dn.toml` sits beside the `.dn` files and holds what is about the build
rather than the domain: for each target, the adapter that fills each port
and the C files holding its native functions. The compiler checks every name
in it against the program.

```toml
[package]
name = "dungeon"
version = "0.1.0"

[target.spectrum_48k]
native = ["platform/spectrum.c"]

[target.spectrum_48k.adapters]
Screen = "SpectrumScreen"
Keyboard = "SpectrumKeyboard"
Storage = "DivmmcStorage"
```

`dn check` works without a `dn.toml`; `dn compile` needs one.

## The domain

The program starts in its `domain`, declared once, outside every context.
Like a function, it has a contract and a body. The contract holds the
context map: every context in the program, with arrows from upstream to
downstream. A context may only use contexts upstream of it, and the arrows
may not form a circle. The contract also imports what the body needs. The
body runs first.

```dn
domain Game {
    contexts {
        Dungeon -> Scoring
        Dungeon -> Platform
    }

    use { SetUpRooms, PlayTurn, FIRST_ROOM, Keyboard } from Dungeon
    use { SetUpScoring } from Scoring

    uses Keyboard
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
```

It only sees what contexts publish, so each context sets up its own starting
data through a service it publishes. The body can pass up any error with `?`;
the program then prints the error's name and stops with exit code 1. A target
can run a service instead, such as a test entry point, with `main = "RunTests"`
under that target in `dn.toml`.

## The generated C

`dn compile` writes four files: the runtime (`dn.h`, `dn_runtime.c`), and the
program (`program.h`, `program.c`), and copies in the target's native C files.
`program.h` lists the native functions' prototypes. Build every `.c` file with
the target's own C compiler.

```sh
cd examples/dungeon/target/dn/test
cc -std=c89 *.c -o dungeon
./dungeon --title
```

- **C89 only.** No `//` comments, no declarations after statements, no
  floating point unless the program uses `Float`, and structs are passed by
  pointer, since cc65 can only pass 1, 2 and 4 byte structs by value.
- **Fixed sizes.** A list holds as many items as the largest `Length` rule
  on any list of its item type (16 when there is none), and every string has
  room for the longest string any rule allows (32 by default). A whole
  number field with a `Range` rule takes the smallest C type that fits.
- **Errors** share one C type, `dn_error`, which holds a code and the error's fields.
- **Events** are queued by commands and kept once the aggregate that raised
  them is saved. When a service ends, its kept events go to the policies;
  events from a failed command, or from a change that was never saved, are dropped.
- **Repositories** keep their aggregates in memory for now. Saving through
  the storage port comes later.
- **Contracts.** Preconditions with an error, and invariants, are always
  checked. Built with `-DDN_DEBUG`, a program also checks plain preconditions
  and every `where` rule: on arguments when a function, command or adapter
  operation starts, on results before they are given back, and on fields
  whenever a value is built or a command ends. A broken rule stops the
  program with a message such as `heal: amount must satisfy Range(1, 50)`.
  Release builds skip these checks.

## Layout

| Crate | Responsibility |
|---|---|
| `dn_syntax` | Lexer, parser and syntax tree |
| `dn_analysis` | Merging files, name resolution, declaration checks and type checking |
| `dn_codegen` | C89 generation for one target |
| `dn_cli` | The `dn` command |

## Development

```sh
cargo test
```

The `dn_codegen` tests generate C for small programs, build it with `cc` in
strict C89 mode with every warning as an error, and run it.
