# Glossary

The shared language for Croft. These terms mean the same thing in the language,
the compiler's code, error messages and conversation. If a new concept needs a
name, add it here first.

Examples come from Logistics, a small cargo shipping system after the one in
Eric Evans' *Domain-Driven Design*. It is the test fixture in
`tests/fixtures/logistics`.

## Structure

### Context
A bounded context: a named boundary inside which every term has one meaning.
Everything in Croft is declared inside a context.

### Publish
The braced list of terms a context lets other contexts use. Aggregates are
never published, only their identities, values and events.

### Use
How a context imports published terms: `use { Edge, Path } from Routing`.

### Context level
Declared directly inside a context rather than inside an aggregate. Values,
enums, constants, constraints, identities, functions, policies, services,
ports and repositories live at context level.

## Syntax

The parts of a declaration, named on one function:

```croft
fn greet(): String where Length(0, 11) = {
    "Hello, World"
}
```

`fn` is the keyword, `greet` the name, `()` the empty parameter list and
`String` the return type. Together they are the signature. `where Length(0, 11)`
constrains the result, so it is part of the contract. Everything after `=` is
the body, here a code block holding one statement, the string literal
`"Hello, World"`.

### Declaration
Something a program names and defines: a context, value, enum, constant,
constraint, identity, entity, aggregate, function, factory, command, policy,
service, port, repository, adapter, translation or the domain. It starts with
its keyword, such as `fn`, `value` or `aggregate`.

### Keyword
A word the language reserves, such as `fn`, `let`, `if`, `match` or `where`.
It can't be used as a name. Words such as `on`, `for`, `from` and `when` are
only special in one place, so they stay ordinary names elsewhere.

### Signature
How code is called, written before its contract: the keyword, the name, any
type parameters, the parameters and the return type, as in
`fn first_or<T>(items: List<T>, fallback: T): T`. A port declares its
operations by signature only.

### Parameter
One named input in a signature, with its type and any constraints:
`deadline: Int where Range(0, 3650)`. A function with none still writes `()`.

### Type parameter
A single capital letter in angle brackets after a name, standing for a type
chosen at each use: the `T` in `first_or<T>`. See Generic.

### Return type
The type after the parameters and `:`, the kind of thing the body gives back.
With no return type it is `Unit`.

### Type
What kind of thing a value is: `Int`, `String`, `List<Leg>`, `Cargo`. A
type can be followed by `where` and constraints wherever it appears on a
field, parameter or return type.

### Expression
Code that works out to a value: `1 + 2`, `cargo.route`, `"Hello, World"`,
a `match`, a call. In Croft nearly everything is an expression, `if` and
`loop` included.

### Literal
A value written out in full: `30`, `2.50`, `"CNHKG"`, `true`, `[a, b]`, `[]`,
`["CNHKG": 4]`, `[:]`.

### Statement
One step of a code block, on its own line: a `let`, a `set`, or an
expression.

### Block
Anything in braces. Each kind holds different things:

- A code block holds statements, run in order, and gives back the value of
  the last. A body, a guard and a loop each hold one:
  `if hit_points == 0 { Defeated }`.
- A contract block sits between a signature and `=`, and holds
  preconditions, constraints on parameters and `calls`.
- A field block holds the fields of a value, entity, aggregate, error or
  event: `value Pair<A, B> { first: A  second: B }`.
- A field value block gives fields their values when making or copying
  one: `Cargo { ... }`, `route with { destination: "NLRTM" }`.
- A variant block holds an enum's variants.
- A match block holds a `match`'s arms.
- A declaration block holds the declarations inside a context, aggregate,
  port, repository or adapter.
- A term block lists the terms a context publishes or uses:
  `use { Edge, Path } from Routing`.
- The contexts block is the domain's context map.

## Data

### Value
A value object: defined only by its fields, equal when its fields are equal,
and never changed. Made with its type name, `RouteSpecification { origin: "CNHKG", destination: "FIHEL", arrival_deadline: 30 }`.

### Generic
A value or function written once for many types, with type parameters that
are single capital letters: `value Pair<A, B> { first: A  second: B }`,
`fn first_or<T>(items: List<T>, fallback: T): T`. Each use with different
types is an instance, such as `Pair<Int, String>` or `first_or<TrackingId>`, which
the compiler makes and checks as if it had been written out by hand, so a
mistake in a template shows where it is used. A call's type parameters come
from what it is given, and from the type it must give back.

### Single-value type
A value around one value of another type, so it can't be mixed up with
other uses of that type: `value Day: Int where Range(0, 3650)`. Made
with `Day(30)`, or a literal where a `Day` is expected, and read with
`day.value`. It has none of the operators of the type it holds, so
`Day(day.value + 1)` makes a new one, and its constraints are checked
there.

### With
Makes a changed copy of a value: `route with { destination: "NLRTM" }`. The original
is untouched. An aggregate is only copied this way inside its own commands and
factories; elsewhere it changes by issuing a command.

### Field
One named, typed part of a value, entity, aggregate or error:
`arrives: Int where Range(0, MAX_DAY)`.

### Enum
A fixed set of named variants, which can carry data: `Held(days: Int)`.

### Const
A named value fixed at compile time, in UPPER_SNAKE_CASE.

### Identity
A distinct type whose only job is to tell entities apart. A `TrackingId` can
never be used where a `HandlingEventId` is expected. Each names what it
identifies, `identity TrackingId for Cargo: Int`, and that aggregate or entity
holds it in one field.

### Entity
Something with an identity that stays the same while its other fields change.
Entities only exist inside the aggregate that owns them.

### Aggregate
The consistency boundary around one or more entities and values. It owns its
entities, invariants, errors, events, factories and commands. It is only made
by its own factories and only changed by its own commands.

## Constraints and invariants

### Constraint
A named, pure check that returns `Bool`. Its first parameter is the value being
checked.

### Range and Length
The built-in constraints. `Range(low, high)` limits a number. `Length(low, high)`
limits the item count of a list or the character count of a string, and
`Length(n)` means exactly n. The compiler also uses them to decide memory size.

### Capacity
How much a string, list or `Bytes` can hold, fixed when the program is
compiled. A string field, variant field, parameter or result with a `Length`
constraint holds that many characters. A string the program builds holds as
many as it can need: a literal is as long as it is, `trim`, `slice` and each
part of `split` or `lines` are no longer than the string they come from, a
joined string is as long as both parts, `repeat` and `pad_left` as long as
the `Range` of their count or width allows, and a function gives back a
string as long as the longest it can return. Each part of `split` or
`lines`, and the result of `trim` or `slice`, borrows its characters from the string it comes from rather than
copying them, unless that string can change before the function returns. A
string with nothing to bound it, such as a parameter with no `Length` or a
`let` that is `set` again later, holds 32 characters, or the longest
`Length` in the program if that is longer. A string too long for where it
goes is cut short; in a debug build that stops the program and says which
field it was. `Bytes` are kept the same way: a field, variant field,
parameter or result with a `Length` holds that many values, a literal holds
as many as it has, `slice` and `set` no more than it came from, `add` one
more, `add_u16` two more, `+` as many as both, and
`to_bytes` as many as the string has characters. A `Bytes` with nothing to
bound it holds 64 values, or the longest `Length` on any `Bytes` if that is
longer.

### Where
Attaches constraints to a type, on a field, parameter or return type:
`deadline: Int where Range(0, 3650)`.

### Each
Applies a constraint to every item in a list: `where Length(0, MAX_LEGS), each GoesSomewhere`.
For constraints about being in that list, not about the item itself.

### Invariant
A constraint on an aggregate as a whole, declared inside it and checked after
every factory and command: `invariant ClaimedAtDestination or ClaimedElsewhere`.

### Error
A named way that working with an aggregate can fail, optionally with fields:
`error NoSuchCargo { tracking_id: TrackingId }`. Errors are part of the domain;
messages shown to the customer are not.

### Precondition
A condition in a contract that must hold for the code to run. With an error,
`delivery.transport_status != Claimed or AlreadyClaimed`, it is a failure the domain expects:
always checked, and handled by the caller like any other failure. Without
one, `!numbers.is_empty`, it is a mistake by the caller: checked in debug
builds, which stop with a message.

### Contract
What a piece of code needs and what it promises, written before its body:
its preconditions and, for services, `calls` in the braces between the
signature and `=`; the `where` constraints on its parameters and result, which
a parameter can have after its type or in the braces (`deadline where
Range(0, 3650)`); and, for
an aggregate, its invariants. The braces are left out when there is nothing
to say. Debug builds check every `where` constraint as the program runs, and
stop with a message naming the constraint that broke. A contract states domain
rules once, where they apply, names
failures in the ubiquitous language, keeps bodies simple, and tells the
compiler how much memory data needs.

## Behaviour

### Factory
The only way to make a new aggregate. Declared inside it, with an optional
contract and a body that gives back the aggregate:
`factory Book(tracking_id: TrackingId, route: RouteSpecification) = Cargo { ... }`.
Called from the aggregate's own context with named arguments,
`Cargo.Book(tracking_id: SAMPLE_CARGO, route: route)?`, it gives back a `Result`
with the aggregate's errors. The new aggregate must meet its invariants.

### Command
The only way to change an aggregate. Declared inside it, with an optional
contract and a body.

### Issue
Sends a command to one aggregate: `issue AssignToRoute(...) on cargo` for a value
you hold, or `issue RecordHandling(...) on Cargo(event.tracking_id)` to load,
change and save by identity. Arguments are always named.

### Event
Something that happened, named in the past tense, declared inside the
aggregate that raises it. Raised in a command body with `emit`. An event
only counts once its aggregate is saved: if the changed aggregate is never
saved, its events are dropped, so policies never react to a change that was
not kept.

### Policy
A reaction to an event that issues a command, often in another aggregate or
context: `policy UpdateDelivery when HandlingEvent.Registered { ... }`.

### Function
A pure calculation that doesn't belong to one value or aggregate:
`fn satisfies(itinerary: Itinerary, route: RouteSpecification): Bool = ...`. With no return
type, a function, service or port operation gives back `Unit`.

### Service
The outer layer, where the program meets the world. Lists the ports it uses,
loads aggregates, issues commands and calls ports.

### Body
Everything after `=`: one expression, or a code block. Bodies are expressions with no `else`. Domain bodies
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
counter's constraints allow, in order; `Range` includes both ends, so that loop runs
100 times. A loop gives back `Unit`, and `?` inside it leaves the whole service.

### Break
Ends the loop it is in. Written alone on its line, usually inside a guard.

### Guard
An `if` without `else`. When its condition holds and its block gives back a
value, that value is returned and the body ends. A block that only takes
steps (`set`, `emit`, `break`, a port call) runs them and the body carries on.
Every body ends with an unconditional line.

### Match
Chooses by enum variant and must handle every variant. A last arm of `_`
handles every variant the arms above it don't: `_ -> 0`.

### `?` and `or`
How failure is handled. `?` passes a failure up when the body's error type
includes it. `or` fails with a more specific error instead. `or` works on a
`Result`, on an `Option` (failing when it is `None`) and on a `Bool` check.
A `Result` that is neither passed up nor the body's result doesn't compile.

### Result
What a body that can fail gives back: `Result<Cargo, Cargo.Error>`. A body
whose last line is a plain value counts as success, so a service returning
`Result<Unit, ShippingError>` can end with a port call.

### Error set
All of an aggregate's errors at once, written `Cargo.Error`. An enum can
`include` error sets and single errors, so one type covers every way a
service can fail. An aggregate with no errors, such as `HandlingEvent`, has
an empty set: it never fails, so any error type can pass it up.

### Built-ins
Functions and methods every program has, with no import. Functions: `Some`,
`None`, `Ok`, `Err`, and the constraints `Range` and `Length`. Strings:
`length`, `is_empty`, and joining with `+`. Numbers: `to_string`. Lists:
`length`, `is_empty`, `first`, `last`, `contains`, `get`, `index_of`, `add`,
`drop`, `take`, `reverse`, `filter`, `map`, `all`, `any`, `count`, `find`,
`sum`, `min_by`, `max_by`, `sort_by`, `sort` for numbers and strings, and
joining with `+`; a list of entities also has `get`, `contains`, `replace` and
`remove` by identity. Options: `is_some`, `is_none`, `map`, `unwrap_or`.
Results: `is_ok`, `is_err`, `map`, `map_err`, `unwrap_or`.

### Map
Keys, each with a value, imported with `use { Map } from Std.Collections`:
`let berths: Map<String, Int> = ["CNHKG": 4, "FIHEL": 2]`, or `[:]` when empty.
`get` gives an `Option` of a key's value, `set` and `remove` give a changed
copy, and `contains`, `length`, `is_empty`, `keys` and `values` read it. Two
maps are equal when they hold the same keys with the same values, in any
order. A map holds as many entries as the largest `Length` constraint on its
type allows, 16 when there is none.

### Bytes
A sequence of bytes, each from 0 to 255, for file contents, saves and network
messages. Written as a list of numbers where `Bytes` is expected:
`let header: Bytes = [0x44, 0x4E, 1]`. It has `length`, `is_empty`, `get`
(an `Option<Int>`), `set`, `add` and `slice`, and joins with `+`:
`header + [0, 1]`. `get_u16` and `add_u16` read and write a number from 0 to
65535 in two bytes, the low byte first, as the Z80 and 6502 keep them;
`get_u16_be` and `add_u16_be` put the high byte first, as the 68000 does. `Std.Text` turns text into bytes with
`to_bytes`, and back with `to_text`. Each `Bytes` is kept in the most values
it can need, as a string is; see Capacity.

### Standard context
A set of built-ins a context imports before using, like any other context:
`use { trim, split } from Std.Text`. `Std.Text` has `contains`,
`starts_with`, `ends_with`, `index_of`, `to_upper`, `to_lower`, `trim`,
`slice`, `split`, `lines`, `replace`, `repeat`, `pad_left`, `pad_right`,
`to_int` and `to_bytes` for strings, `join` for a list of strings, and
`to_text` for bytes. `Std.Numbers`
has `Decimal`, `max`, `min` and `abs`. `Std.Collections` has `Map`.
Each file of a context sees what any of them imports. Standard contexts need
no arrow in the domain's `contexts`.

Some standard contexts are written in Croft and join the program when it
imports from them. `Std.Files` has the `Files` port, with `read`, `write`,
`append`, `exists`, `remove`, `list`, which gives the names in a folder in
byte order, each folder's ending in `/`, `make_folder`, which makes each
folder on the way, and `copy`, for a file of any size, its errors `NotFound`, `NoRoom` and
`Unreadable`, the enum `FileError` that includes them, and the `StdioFiles`
adapter, which uses C's stdio: `Files = "StdioFiles"` in `croft.toml`. Its
`list` and `make_folder` need POSIX, so they fail with `Unreadable` on
machines without it.
`Std.Http` has the `Http` port, with `get`, `post`, `put` and `delete`, each
giving a `Response` (its `status`, its `headers` as a map with names in
lower case, and its `body`) or an `HttpError`:
`Unreachable`, `TimedOut` or `TooLarge`. Its `CurlHttp` adapter uses libcurl,
which also handles HTTPS, so a program that binds it is linked with `-lcurl`.

## The outside world

### Port
What the domain needs from the outside world, by signature only.

### Repository
A port for keeping one aggregate. `repository Cargos for Cargo` on its own
has `find` and `save`: `find` fails with the aggregate's errors or a
`StorageError`, and `save` with a `StorageError`. Or it declares its own
operations, named in the ubiquitous language:
`repository Cargos for Cargo { fn awaiting_customs(): Result<List<Cargo>, StorageError> }`.
Each operation takes or gives back only that aggregate, its identity or plain
values, and can fail with a `StorageError`. Each target binds the repository
in `croft.toml` to an adapter written for the repository itself, to keep
aggregates its own way, or to an adapter for `Storage`, which keeps the bytes
the compiler makes from each aggregate under a key such as `carg12` for
cargo 12: `MemoryStorage`, the default, `FileStorage`, or one of the
program's own. `Storage` can only provide a find by identity, giving an
`Option` or failing with `NotFound`, a save and a remove.

### Storage
The built-in port that repositories keep their aggregates through: `load`
gives the bytes kept under a key, or `None`, `save` keeps them, and `remove`
drops them. `MemoryStorage` and `FileStorage` are its built-in adapters.

### StorageError
What loading or saving an aggregate can fail with, built in to every
program with a repository: `StorageFull`, `StorageDamaged` (written by a
different version of the program, or damaged) and `StorageUnavailable`.

### Arguments
The port in `Std.Console` that gives a program its command-line arguments,
with no import needed. A service lists it in `calls`, then reads
`arguments.all()` for a `List<String>`. Every target supplies it; targets
without a command line, such as the ZX Spectrum, give an empty list.

### Console
The port in `Std.Console` that prints a line, with no import needed:
`console.print("hello")` in a service or domain that lists it in `calls`.
Every target supplies it, so `croft.toml` names no adapter for it. A
program's own port called `Console` replaces it.

### Adapter
Implements a port, usually in a `Platform` context: `adapter FixedClock for
Clock = { ... }`. Must provide every operation the port declares. Which
targets use it is chosen in `croft.toml`, so one adapter can serve several.

### Starting value
What an adapter's field holds when the program starts, written after its
type and constraints: `voyages: List<String> where Length(0, 8) = ["V100", "V200"]`. Every
adapter field needs one; no other field has one.

### Native
Calls a C function from an adapter: `native terminal_next_report()`.
Only allowed inside adapters.

### Translation
The anti-corruption layer: maps one context's term to another's, such as
Routing's edges to Shipping's legs: `translation LegFrom(edge: Edge): Leg`.

## Compiler

### Source file
A file of Croft code, ending in `.croft`, or in `.cro` on machines whose file
names are limited to 8.3. Either is read in capitals too, as in `CARGO.CRO`.

### Project
A folder of source files compiled together as one program, with a `croft.toml`.

### Project file
`croft.toml`, beside the source files. Like Rust's `Cargo.toml`, it holds what
is about the build rather than the domain: the program's name, and for each
target the adapter that fills each port (`Clock = "SystemClock"`), the C
files holding its native functions, and optionally a `main` service to run
instead of the domain.

### Domain
The whole program: `domain Logistics { ... } = { ... }`, one per program, outside
every context. Like a function, it has a contract and a body. The contract
holds the context map, imports what the body needs from the contexts with
`use`, and names the ports it calls with `calls`. The body runs
first: it sets up each context's starting data through services those
contexts publish, then runs the program. It can pass up any error with `?`;
the program then prints the error's name and stops with exit code 1.

### Context map
The `contexts` block of the domain: every context, and arrows from upstream
to downstream, `Routing -> Shipping`. A context may only `use` contexts
directly upstream of it, and the arrows may not go round in a circle. A
context with no relationships is written on its own.

### Main
A service a target runs instead of the domain, named by `main` under that
target in `croft.toml`, such as a test entry point. It takes no parameters.

### Target
A machine to build for, such as `spectrum_48k`, `atari_st`, `desktop` or `test`.

### Check
Reading, parsing and checking a project without generating code (`croft check`).

### Compile
Checking a project, generating code and building it (`croft compile`).

### Diagnostic
An error the compiler reports, with its file, line and column.
