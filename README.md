```
    )
  _||_____
  /  \\\\\\\\
/____\\\\\\\\\
| [] |  _  | 
|____|_| |_|
```

# Croft

Croft is a domain language, declarative structure, imperative behaviour. The
domain model is the program: you write it with domain-driven design's building
blocks (contexts, values, entities, aggregates, commands, events, ports and
adapters) and the compiler checks the constraints you attach to them. Croft is
designed to compile to portable C, so you can ship cross-platform.

## Usage

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

```sh
cargo install --path crates/croft_cli

croft check . # compiler check
croft compile . --target desktop # check and compile to C
croft run . --target desktop # check, compile and run C
```

Every `.c` file in the output folder is part of the program, for example
`cc -std=c89 target/croft/desktop/*.c -o logistics`. Besides `program.c` and
`croft_runtime.c`, it holds your native files and the standard adapters the
target uses, such as `croft_files.c` for `StdioFiles`, and `croft_http.c` for
`CurlHttp`, which needs `-lcurl` when linking.

`croft run . --target desktop` does all of this in one step: it compiles, builds
the C with `$CC` (or `cc`), adding `-lcurl` when the target uses `CurlHttp`, and
runs the program from `target/croft/desktop/<package name>`. Arguments after
`--` go to the program.

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
on hover, clicking takes you straight to the definition.

- VS Code: [editors/vscode](editors/vscode), with highlighting from a TextMate
  grammar.
- Zed: [editors/zed](editors/zed), with highlighting from the Tree-sitter
  grammar in [editors/tree-sitter-croft](editors/tree-sitter-croft).

## This repository

| Crate            | Responsibility                                                       |
| ---------------- | -------------------------------------------------------------------- |
| `croft_syntax`   | Lexer, parser and syntax tree                                        |
| `croft_analysis` | Merging files, name resolution, declaration checks and type checking |
| `croft_codegen`  | C89 generation for one target                                        |
| `croft_cli`      | The `croft` command, and the language server, `croft lsp`            |

## Development

```sh
cargo test
```

The `croft_codegen` tests generate C for small programs, build it with `cc` in
strict C89 mode with every warning as an error, and run it.
