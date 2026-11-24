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

## Status

The design is still being decided. The terms are in
[docs/glossary.md](docs/glossary.md).

The compiler in this repository is an early trial. It reads an older subset of
the language (`struct`, `enum`, `const`, `constraint`, `port`, `adapter` and
`fn` with `requires`/`ensures`) and compiles it to Rust. Next, it moves to the
current design and a C backend.

## Usage

```sh
cargo install --path crates/dn_cli

dn check .        # check every .dn file in the folder
dn compile .      # check, generate code in target/dn/ and build it
dn compile . --release
dn compile . --no-build
```

Try it on the example:

```sh
dn compile examples/invoicing
```

## Layout

| Crate | Responsibility |
|---|---|
| `dn_syntax` | Lexer, parser and syntax tree |
| `dn_analysis` | Name resolution, type checking and naming rules |
| `dn_codegen` | Code generation |
| `dn_cli` | The `dn` command |

## Development

```sh
cargo test
```

The `dn_codegen` tests compile the example's generated code with
`rustc -D warnings` and run behaviour tests against it.
