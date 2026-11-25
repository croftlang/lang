# Glossary

The shared language for dn. These terms mean the same thing in the language,
the compiler's code, error messages and conversation. If a new concept needs a
name, add it here first.

Examples come from Dungeon, the small game used throughout the design.

## Structure

### Context
A bounded context: a named boundary inside which every term has one meaning.
Everything in dn is declared inside a context.

### Publish
The braced list of terms a context lets other contexts use. Aggregates are
never published, only their identities, values and events.

### Use
How a context imports published terms: `use { EnemyId, Room.EnemyDefeated } from Dungeon`.

### Context level
Declared directly inside a context rather than inside an aggregate. Values,
enums, constants, constraints, identities, functions, policies, services,
ports and repositories live at context level.

## Data

### Value
A value object: defined only by its fields, equal when its fields are equal,
and never changed. Made with its type name, `Position { x: 10, y: 20 }`.

### With
Makes a changed copy of a value: `start with { x: start.x + 1 }`. The original
is untouched.

### Field
One named, typed part of a value, entity, aggregate or error:
`hit_points: Int where Range(0, 100)`.

### Enum
A fixed set of named variants, which can carry data: `Coins(count: Int)`.

### Const
A named value fixed at compile time, in UPPER_SNAKE_CASE.

### Identity
A distinct type whose only job is to tell entities apart. An `EnemyId` can
never be used where a `RoomId` is expected.

### Entity
Something with an identity that stays the same while its other fields change.
Entities only exist inside the aggregate that owns them.

### Aggregate
The consistency boundary around one or more entities and values. It owns its
entities, invariants, errors, events and commands, and is only changed by its
own commands.

## Rules

### Constraint
A named, pure rule that returns `Bool`. Its first parameter is the value being
checked.

### Range and Length
The built-in constraints. `Range(low, high)` limits a number. `Length(low, high)`
limits the item count of a list or the character count of a string, and
`Length(n)` means exactly n. The compiler also uses them to decide memory size.

### Where
Attaches constraints to a type, on a field, parameter or return type:
`damage: Int where Range(1, 20)`.

### Each
Applies a constraint to every item in a list: `where Length(0, 16), each Alive`.
For rules about being in that list, not about the item itself.

### Invariant
A rule about an aggregate as a whole, declared inside it and checked after
every command: `invariant ExitClear or ExitBlocked`.

### Error
A named way that working with an aggregate can fail, optionally with fields:
`error EnemyMissing { enemy_id: EnemyId }`. Errors are part of the domain;
messages shown to the player are not.

### Precondition
A condition in a contract that must hold for the code to run. With an error,
`in_stock > 0 or OutOfStock { item }`, it is a failure the domain expects:
always checked, and handled by the caller like any other failure. Without
one, `!numbers.is_empty`, it is a mistake by the caller: checked in debug
builds, which stop with a message.

### Contract
What a piece of code needs and what it promises, written before its body:
its preconditions and, for services, `uses` in the braces between the
signature and `=`; the `where` rules on its parameters and result; and, for
an aggregate, its invariants. The braces are left out when there is nothing
to say. A contract states domain rules once, where they apply, names
failures in the ubiquitous language, keeps bodies simple, and tells the
compiler how much memory data needs.

## Behaviour

### Command
The only way to change an aggregate. Declared inside it, with an optional
contract and a body.

### Issue
Sends a command to one aggregate: `issue Attack(...) on room` for a value you
hold, or `issue Reset on Scoreboard(MAIN_SCOREBOARD)` to load, change and save
by identity. Arguments are always named.

### Event
Something that happened, named in the past tense, declared inside the
aggregate that raises it. Raised in a command body with `emit`. An event
only counts once its aggregate is saved: if the changed aggregate is never
saved, its events are dropped, so policies never react to a change that was
not kept.

### Policy
A reaction to an event that issues a command, often in another aggregate or
context.

### Function
A pure calculation that doesn't belong to one value or aggregate:
`fn distance(start: Position, end: Position): Int = ...`.

### Service
The outer layer, where the program meets the world. Lists the ports it uses,
loads aggregates, issues commands and calls ports.

### Body
Everything after `=`. Bodies are expressions with no `else`. Domain bodies
(commands, functions, constraints, translations) have no loops, so they
always finish; list methods such as `map` and `sum` do their iterating.

### Let
Names a value that never changes. Works in every body.

### List literal
`[a, b, c]` makes a list of exactly those items, and `[]` an empty one,
whose item type comes from where it is used.

### Set
Changes an aggregate's field in a command, an adapter's field, or a
service's own `let` name. Not allowed anywhere else.

### Loop
Repeats a block, in services and adapters only. `loop { ... }` runs until
`break`. `loop (let i where Range(0, 99)) { ... }` runs once for each value the
counter's rules allow, in order; `Range` includes both ends, so that loop runs
100 times. A loop gives back `Unit`, and `?` inside it leaves the whole service.

### Break
Ends the loop it is in. Written alone on its line, usually inside a guard.

### Guard
An `if` without `else`. When its condition holds and its block gives back a
value, that value is returned and the body ends. A block that only takes
steps (`set`, `emit`, `break`, a port call) runs them and the body carries on.
Every body ends with an unconditional line.

### Match
Chooses by enum variant and must handle every variant.

### `?` and `or`
How failure is handled. `?` passes a failure up when the body's error type
includes it. `or` fails with a more specific error instead. `or` works on a
`Result`, on an `Option` (failing when it is `None`) and on a `Bool` check.
A `Result` that is neither passed up nor the body's result doesn't compile.

### Result
What a body that can fail gives back: `Result<Room, Room.Error>`. A body
whose last line is a plain value counts as success, so a service returning
`Result<Unit, GameError>` can end with a port call.

### Error set
All of an aggregate's errors at once, written `Room.Error`. An enum can
`include` error sets and single errors, so one type covers every way a
service can fail.

### Built-ins
Functions and methods every program has. Functions: `max`, `min`, `abs`,
`Some`, `None`, `Ok`, `Err`, and the constraints `Range` and `Length`.
Strings: `length`, `is_empty`, `contains`, `starts_with`, `ends_with`,
`to_upper`, `to_lower`, `trim`, `slice`, `split`, `to_int`. Numbers:
`to_string`. Lists: `length`, `is_empty`, `first`, `last`, `contains`, `get`,
`add`, `drop`, `take`, `filter`, `map`, `all`, `any`, `count`, `find`, `sum`,
`min_by`, `max_by`; a list of entities also has `get`, `contains`, `replace`
and `remove` by identity. Options: `is_some`, `is_none`, `map`, `unwrap_or`.

## The outside world

### Port
What the domain needs from the outside world, by signature only.

### Repository
A port for loading and saving one aggregate, built on another port:
`repository Rooms for Room in Storage`. The compiler writes `find`, which
fails with the aggregate's errors, and `save`, which fails with the storage
port's error.

### Arguments
The built-in port that gives a program its command-line arguments. A
service lists it in `uses` and calls `arguments.all()` for a
`List<String>`. Every target supplies it; targets without a command line,
such as the ZX Spectrum, give an empty list.

### Adapter
Implements a port, usually in a `Platform` context: `adapter NullScreen for
Screen { ... }`. Must provide every operation the port declares. Which
targets use it is chosen in `dn.toml`, so one adapter can serve several.

### Starting value
What an adapter's field holds when the program starts, written after its
type and rules: `keys: List<Key> where Length(0, 32) = [Right, Fire]`. Every
adapter field needs one; no other field has one.

### Native
Calls a C function from an adapter: `native sprite_draw(sprite, at.x, at.y)`.
Only allowed inside adapters.

### Translation
The anti-corruption layer: maps one context's term to another's.

## Compiler

### Project
A folder of `.dn` files compiled together as one program, with a `dn.toml`.

### Project file
`dn.toml`, beside the `.dn` files. Like Rust's `Cargo.toml`, it holds what
is about the build rather than the domain: the program's name, and for each
target the adapter that fills each port (`Screen = "SpectrumScreen"`), the C
files holding its native functions, and optionally a `main` service to run
instead of the domain.

### Domain
The whole program: `domain Game { ... } = { ... }`, one per program, outside
every context. Like a function, it has a contract and a body. The contract
holds the context map, imports what the body needs from the contexts with
`use`, and names the ports it calls with `uses`. The body runs
first: it sets up each context's starting data through services those
contexts publish, then runs the program. It can pass up any error with `?`;
the program then prints the error's name and stops with exit code 1.

### Context map
The `contexts` block of the domain: every context, and arrows from upstream
to downstream, `Dungeon -> Scoring`. A context may only `use` contexts
directly upstream of it, and the arrows may not go round in a circle. A
context with no relationships is written on its own.

### Main
A service a target runs instead of the domain, named by `main` under that
target in `dn.toml`, such as a test entry point. It takes no parameters.

### Target
A machine to build for, such as `spectrum_48k`, `atari_st`, `desktop` or `test`.

### Check
Reading, parsing and checking a project without generating code (`dn check`).

### Compile
Checking a project, generating code and building it (`dn compile`).

### Diagnostic
An error the compiler reports, with its file, line and column.
