# sp

sp is an experimental specification language. You write the structure of a
program in it: structs, enums, constants, constraints, ports and function
contracts. You don't write function bodies. Those are meant to be generated
by AI and checked against the contracts. sp compiles to Rust, and Rust
compiles to machine code.

Body generation isn't built yet. For now, bodies are generated as `todo!()`
placeholders. Everything else, from parsing to a built crate, works.

```sp
constraint NonEmpty<T>(list: List<T>) = list.length > 0

struct Invoice {
    lines: List<Line> is NonEmpty
    issue_day: Int
    due_day: Int
} is DueAfterIssue

fn settle_invoice(id: InvoiceId) -> Result<Receipt, SettleError> {
    uses InvoiceRepository, PaymentGateway
    requires NotBlank(id.value)
    description "Charges the invoice total and marks it as paid."
}
```

## Usage

```sh
cargo install --path crates/sp_cli

sp check .        # check every .sp file in the folder
sp compile .      # check, generate Rust in target/sp/ and build it
sp compile . --release
sp compile . --no-build   # generate the crate without running cargo
```

Try it on the example:

```sh
sp compile examples/invoicing
```

## What gets generated

| sp | Rust |
|---|---|
| `struct` | `struct` with `impl Validate` checking its constraints |
| `enum` | `enum` |
| `const` | `const` |
| `constraint` | `fn ... -> bool` |
| `port` | `trait` |
| `adapter` | `struct` implementing the port's trait |
| `fn` | a public function that runs contract checks in debug builds, around a private `_body` function |

## Layout

| Crate | Responsibility |
|---|---|
| `sp_syntax` | Lexer, parser and syntax tree |
| `sp_analysis` | Name resolution, type checking and naming rules |
| `sp_codegen` | Rust and Cargo manifest generation |
| `sp_cli` | The `sp` command |

## Docs

- [Glossary](docs/glossary.md): the agreed terms for the language and compiler

## Development

```sh
cargo test
```

The `sp_codegen` tests compile the example's generated Rust with
`rustc -D warnings` and run behaviour tests against it.
