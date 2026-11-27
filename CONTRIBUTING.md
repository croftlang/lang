# Contributing

## Introduction

Thanks for your interest in Croft! Contributions of all kinds are welcome,
whether you're reporting bugs, suggesting language features, improving the
documentation, or sharing the project with others.

Please read the [Code of Conduct](CODE_OF_CONDUCT.md) before taking part.

## Ways to Contribute

### Report Issues

Found a bug or have a suggestion?
[Create an issue](https://github.com/croftlang/lang/issues/new/choose) using
one of our templates, or a blank issue for anything else:

- **Bug Report** - The compiler, the generated C or the language server isn't
  doing what you expected
- **Documentation Issue** - Reference docs that are missing, unclear or wrong
- **Blank issue** - Feature requests, questions and discussions

When reporting issues:

- Include the Croft commit or version you built from
- Name the target you compiled for (`desktop`, `spectrum_48k`, ...) and the C
  compiler you built the output with
- Provide the smallest `.croft` program and `croft.toml` that shows the problem
- Paste the compiler's full output, or the C compiler's errors
- Describe what you expected against what actually happened

### Submit Pull Requests

We love pull requests! Here's how to contribute code:

#### Getting Started

```sh
# Fork and clone the repository.
git clone https://github.com/YOUR_USERNAME/lang.git croft

cd croft

# Run the tests.
cargo test --workspace

# Install the croft command from your checkout.
cargo install --path crates/croft_cli
```

#### Pull Request Guidelines

- **Keep it focused** - One feature or fix per PR
- **Write tests** - New language features and fixes should come with a test,
  usually a program under `crates/croft_codegen/tests/programs`
- **Follow the style** - Run `cargo fmt --all` and
  `cargo clippy --workspace --all-targets -- -D warnings` before committing
- **Update docs** - If you change the language, open a matching pull request on
  [croftlang/docs](https://github.com/croftlang/docs), which holds the reference
- **Update the editors** - If you change the syntax, update the Tree-sitter
  grammar in [editors/tree-sitter-croft](editors/tree-sitter-croft) and the
  TextMate grammar in [editors/vscode](editors/vscode)
- **Portable C** - Generated C must build in strict C89 with every warning as
  an error, and should not assume a desktop machine

#### Code Philosophy

Croft follows "the domain model is the program". The compiler should be as
plain as the programs it reads:

- Prefer readable code over clever code
- Plain, conventional names over clever or metaphorical ones
- Early returns over deep nesting
- Blank lines between groups of statements in a function body
- Errors that tell the reader what went wrong and where

#### Commit Messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org),
with a type, an optional scope and a clear summary:

```txt
✅ Good:
- "fix(lsp): report unknown built-ins inside domains"
- "perf: work out whole numbers in 16 bits where every value fits"
- "docs: name the parts of a declaration in the glossary"

❌ Avoid:
- "Fix bug"
- "Update stuff"
- "WIP"
```

Common types are `feat`, `fix`, `perf`, `refactor`, `docs` and `test`.

### Star the Project

The simplest way to show support! A star on
[GitHub](https://github.com/croftlang/lang) helps others discover Croft.

### Use the Project

The best contribution is writing Croft:

- Model a domain you know with it
- Try it on the targets you care about, old machines included
- Report real-world use cases and pain points
- Provide feedback on the language and its error messages

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs) stable, with `rustfmt` and `clippy`
- A C compiler (`cc`), used by the tests to build the generated C
- libcurl and its headers, for the `Std.Http` tests
- [Node](https://nodejs.org), only if you work on the editor extensions
- Git

### Project Structure

```txt
croft/
├── crates/
│   ├── croft_syntax/      # Lexer, parser and syntax tree
│   ├── croft_std/         # Standard contexts written in Croft, and their adapters' C
│   ├── croft_analysis/    # Merging files, name resolution and type checking
│   ├── croft_codegen/     # C89 generation for one target
│   ├── croft_format/      # The formatter, croft fmt
│   └── croft_cli/         # The croft command and the language server
├── docs/
│   ├── glossary.md        # The names of the language's parts
│   └── reference/         # The language reference
├── editors/
│   ├── tree-sitter-croft/ # Tree-sitter grammar
│   ├── vscode/            # VS Code extension
│   └── zed/               # Zed extension
├── examples/              # Example programs
└── tests/fixtures/        # Larger programs, such as logistics
```

### Running Tests

```sh
# Run all tests.
cargo test --workspace

# Run one crate's tests.
cargo test -p croft_codegen
```

The `croft_codegen` tests generate C for small programs, build it with `cc` in
strict C89 mode with every warning as an error, and run it.

### Code Quality

```sh
# Format code.
cargo fmt --all

# Lint code.
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs both, along with the tests, on every pull request.

## Licence

Croft is licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE)
or the [MIT licence](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Croft by you, as defined in the Apache-2.0 licence, shall be
dual licensed as above, without any additional terms or conditions.

## Questions?

- Read the [reference](https://croftlang.com/learn)
- Browse [existing issues](https://github.com/croftlang/lang/issues)
- Open a [new issue](https://github.com/croftlang/lang/issues/new)
