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
the application has an adapter for every port its services use.

It also type-checks bodies. Every expression gets a type, failures must be
handled with `?` or `or`, `match` must handle every variant, and each kind of
body can only do what its kind allows: only commands `set` fields and `emit`
events, only services and adapters loop, and only adapters call native code.

There is no C backend yet, so `dn compile` stops after the checks.

## Usage

```sh
cargo install --path crates/dn_cli

dn check .        # check every .dn file in the folder and its subfolders
```

Try it on the example, a small dungeon game split across ten files:

```sh
dn check examples/dungeon
```

## Layout

| Crate | Responsibility |
|---|---|
| `dn_syntax` | Lexer, parser and syntax tree |
| `dn_analysis` | Merging files, name resolution, declaration checks and type checking |
| `dn_cli` | The `dn` command |

## Development

```sh
cargo test
```
