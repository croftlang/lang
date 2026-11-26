<img src="./branding/banner.png" alt="Croft" width="100%">

This repository contains the source code for the Croft programming language;
its compiler, standard library and language server.

![CI](https://github.com/croftlang/lang/actions/workflows/ci.yml/badge.svg)

## Why Croft?

- **Clarity:** The domain model _is_ the code, not just a diagram beside it.
  Your contexts, aggregates, commands and events outline what your program is
  about, and a command's contract houses the business rules themselves.
- **Reliability:** Constraints such as `where Length(0, 32)` and command
  contracts are checked by the compiler, so broken rules show up before runtime,
  and every type has a known size.
- **Portability:** Croft compiles to C89, needing no additional runtime, so it
  runs wherever a C compiler does, from modern desktops to the ZX Spectrum and
  Atari ST. Ports and adapters make the program the same on every machine, and
  `croft.toml` configures what connects to them.

## Getting started

```croft
context Greeter {
  publish { greet }

  fn greet(): String where Length(0, 11) = {
    "Hello, World"
  }
}

domain Application {
  contexts {
    Greeter
  }

  use { greet } from Greeter

  calls Console
} = {
  let output = greet() 

  console.print(output) // Hello, World
}

// crof.toml
// [package]
// name = "hello"
//
// [target.desktop]
```

Source files end in `.croft`, or in `.cro` on machines whose file names are
limited to 8.3. A project can mix the two, and the compiler reads either in
capitals too, as in `CARGO.CRO`.
ƒ
```sh
cargo install --path crates/croft_cli

croft check . # compiler check
croft compile . --target desktop # check and compile to C
croft run . --target desktop # check, compile and run C
croft size . --target spectrum_48k # memory each type and function takes
croft fmt . # lay out the source files in the Croft style
```

Try it on the test fixture, a small cargo shipping system after the one in Eric
Evans' _Domain-Driven Design_, split across thirteen files:

```sh
croft check tests/fixtures/logistics
croft compile tests/fixtures/logistics --target desktop
```

## Reference

For full reference documentation, see [./docs/reference](./docs/reference)

## Editors

`croft lsp` runs a language server, showing you errors as you type, without
needing to run the compiler. It also shows you function signatures and comments
on hover, clicking takes you straight to the definition, and it formats a file
the same way as `croft fmt`.

- VS Code: [editors/vscode](editors/vscode), with highlighting from a TextMate
  grammar.
- Zed: [editors/zed](editors/zed), with highlighting from the Tree-sitter
  grammar in [editors/tree-sitter-croft](editors/tree-sitter-croft).

## This repository

| Crate            | Responsibility                                                       |
| ---------------- | -------------------------------------------------------------------- |
| `croft_syntax`   | Lexer, parser and syntax tree                                        |
| `croft_std`      | Standard contexts written in Croft, beside the C of their adapters   |
| `croft_analysis` | Merging files, name resolution, declaration checks and type checking |
| `croft_codegen`  | C89 generation for one target                                        |
| `croft_format`   | The formatter, `croft fmt`                                           |
| `croft_cli`      | The `croft` command, and the language server, `croft lsp`            |

## Development

```sh
cargo test
```

The `croft_codegen` tests generate C for small programs, build it with `cc` in
strict C89 mode with every warning as an error, and run it.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Licence

Croft is licensed under either of the
[Apache License, Version 2.0](LICENSE-APACHE) or the [MIT licence](LICENSE-MIT),
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Croft by you, as defined in the Apache-2.0 licence, shall be
dual licensed as above, without any additional terms or conditions.
