# b

b is a domain language. The domain model is the program: you write it with
domain-driven design's building blocks (contexts, values, entities,
aggregates, commands, events, ports and adapters) and the compiler checks the
constraints you attach to them. b is designed to compile to portable C, so one
program can run on a ZX Spectrum, an Atari ST, Windows, macOS and Linux.

```b
context Shipping {
    aggregate Cargo {
        identity tracking_id: TrackingId
        route: RouteSpecification
        itinerary: Itinerary
        delivery: Delivery

        invariant ClaimedAtDestination or ClaimedElsewhere

        error AlreadyClaimed
        error ClaimedElsewhere

        // Works out the delivery again after the cargo is handled.
        command RecordHandling(activity: HandlingActivity) {
            delivery.transport_status != Claimed or AlreadyClaimed
        } = {
            let misdirected = !expects(itinerary, activity)
            set delivery = Delivery {
                transport_status: status_after(activity.kind)
                last_location: Some(activity.location)
                current_voyage: voyage_after(activity)
                misdirected: misdirected
            }
            emit Handled { activity }
            if misdirected {
                emit Misdirected { location: activity.location }
            }
        }
    }
}
```

## Splitting a context across files

A context can be written inline, in braces, or opened with a header line at
the top of a file. A header claims the rest of the file. Add an `aggregate`
header to write part of one aggregate:

```b
// shipping/cargo/commands.b
context Shipping
aggregate Cargo

command AssignToRoute(chosen: Itinerary) = {
    ...
}
```

The compiler joins every file that opens the same context, and every part of
the same aggregate, before checking. This keeps each file short.

## Status

The design is still being decided. The terms are in
[docs/glossary.md](docs/glossary.md).

The compiler reads the current syntax and checks declarations: every name
resolves, `where` clauses and invariants name real constraints and errors, aggregates
and entities have one identity, adapters provide their port's operations, and
`b.toml` gives every port its services use an adapter on each target.

It also type-checks bodies. Every expression gets a type, failures must be
handled with `?` or `or`, `match` must handle every variant, or end with `_`, and each kind of
body can only do what its kind allows: only commands `set` fields and `emit`
events, only services and adapters loop, and only adapters call native code.
Generic values and functions are checked as each instance the program uses,
such as `Pair<Int, String>`, and each instance becomes C of its own.

`b compile` writes portable C89 for one target. It has been tested with gcc
and clang in strict C89 mode; building with cc65 and z88dk is next.

## Usage

```sh
cargo install --path crates/b_cli

b check .                       # check every .b file in the folder and its subfolders
b compile . --target desktop    # check, then write C to target/b/desktop/
```

Every `.c` file in the output folder is part of the program, for example
`cc -std=c89 target/b/desktop/*.c -o logistics`. Besides `program.c` and
`b_runtime.c`, it holds your native files and the standard adapters the
target uses, such as `b_files.c` for `StdioFiles`, and `b_http.c` for
`CurlHttp`, which needs `-lcurl` when linking.

Try it on the test fixture, a small cargo shipping system after the one in
Eric Evans' *Domain-Driven Design*, split across thirteen files:

```sh
b check tests/fixtures/logistics
b compile tests/fixtures/logistics --target desktop
```

## Editors

`b lsp` runs a language server, which editors start for you. It shows
errors as you type, a declaration's signature and doc comment on hover,
and goes to where a name is declared, including fields, port operations
and the standard library. It completes names in scope, and after a `.`
the fields and operations of a value or the variants of an enum.

- VS Code: [editors/vscode](editors/vscode), with highlighting from a TextMate grammar.
- Zed: [editors/zed](editors/zed), with highlighting from the Tree-sitter grammar in
  [editors/tree-sitter-b](editors/tree-sitter-b).

## The project file

`b.toml` sits beside the `.b` files and holds what is about the build
rather than the domain: for each target, the adapter that fills each port,
where each repository keeps its aggregates, and the C files holding native
functions. The compiler checks every name in it against the program.

```toml
[package]
name = "logistics"
version = "0.1.0"

[target.desktop]
native = ["platform/desktop.c"]

[target.desktop.adapters]
HandlingReports = "TerminalReports"
Clock = "SystemClock"
Files = "StdioFiles"
Cargos = "FileStorage"
HandlingEvents = "FileStorage"
```

`b check` works without a `b.toml`; `b compile` needs one.

## The domain

The program starts in its `domain`, declared once, outside every context.
Like a function, it has a contract and a body. The contract holds the
context map: every context in the program, with arrows from upstream to
downstream. A context may only use contexts upstream of it, and the arrows
may not form a circle. The contract also imports what the body needs. The
body runs first.

```b
domain Logistics {
    contexts {
        Routing -> Shipping
        Shipping -> Platform
    }

    use { SetUpBooking, RouteCargo, RegisterReports, Track, SAMPLE_CARGO } from Shipping
} = {
    SetUpBooking()?
    RouteCargo(SAMPLE_CARGO)?
    RegisterReports()?
    Track(SAMPLE_CARGO)?
}
```

It only sees what contexts publish, so each context sets up its own starting
data through a service it publishes. The body can pass up any error with `?`;
the program then prints the error's name and stops with exit code 1. A target
can run a service instead, such as a test entry point, with `main = "RunTests"`
under that target in `b.toml`.

## The generated C

`b compile` writes four files: the runtime (`b.h`, `b_runtime.c`), and the
program (`program.h`, `program.c`), and copies in the target's native C files.
`program.h` lists the native functions' prototypes. Build every `.c` file with
the target's own C compiler.

```sh
cd tests/fixtures/logistics/target/b/desktop
cc -std=c89 *.c -o logistics
printf '1 1 receive CNHKG\n2 1 load CNHKG V100\n' | ./logistics
```

- **C89 only.** No `//` comments, no declarations after statements, no
  floating point unless the program uses `Float`, and structs are passed by
  pointer, since cc65 can only pass 1, 2 and 4 byte structs by value.
- **Fixed sizes.** A list holds as many items as the largest `Length` constraint
  on any list of its item type (16 when there is none). A string field with a
  `Length` constraint takes just that many characters; every other string has
  room for 32, or for the longest `Length` in the program if that is longer.
  Text that doesn't fit is cut short, and a debug build stops and says where.
  A whole number field with a `Range` constraint takes the smallest C type
  that fits.
- **Only what's called.** `b_runtime.c` holds just the runtime functions the
  program reaches. cc65 and z88dk link a whole C file at once, so anything
  left in would cost memory on an 8-bit machine.
- **Errors** share one C type, `b_error`, which holds a code and the error's fields.
- **Events** are queued by commands and kept once the aggregate that raised
  them is saved. When a service ends, its kept events go to the policies;
  events from a failed command, or from a change that was never saved, are dropped.
- **Repositories** turn each aggregate into bytes, the hash of its layout
  and then every field, numbers as four bytes least significant first, so a
  save reads the same on every machine. The adapter for `Storage` that
  `b.toml` binds keeps them under a key such as `carg12`, for cargo 12: in memory by
  default, or with `FileStorage` in `carg12.dat` through the target's
  `Files` adapter. Bytes from a different version of the program load as
  `StorageDamaged`.
- **Contracts.** Preconditions with an error, and invariants, are always
  checked. Built with `-DB_DEBUG`, a program also checks plain preconditions
  and every `where` constraint: on arguments when a function, command or adapter
  operation starts, on results before they are given back, and on fields
  whenever a value is built or a command ends. A broken constraint stops the
  program with a message such as `heal: amount must satisfy Range(1, 50)`.
  Release builds skip these checks.

## Layout

| Crate | Responsibility |
|---|---|
| `b_syntax` | Lexer, parser and syntax tree |
| `b_analysis` | Merging files, name resolution, declaration checks and type checking |
| `b_codegen` | C89 generation for one target |
| `b_cli` | The `b` command, and the language server, `b lsp` |

## Development

```sh
cargo test
```

The `b_codegen` tests generate C for small programs, build it with `cc` in
strict C89 mode with every warning as an error, and run it.
