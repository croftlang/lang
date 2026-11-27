<img src="./branding/banner.png" alt="Croft" width="100%">

This repository contains the source code for the Croft programming language;
its compiler, standard library and language server.

![CI](https://github.com/croftlang/lang/actions/workflows/ci.yml/badge.svg)

## Why Croft?

- **Clarity:** The domain model _is_ the code, not just a diagram beside it.
  Your contexts, aggregates, commands and events outline what your program is
  about, and a command's contract houses the business rules themselves.
- **Reliability:** Constraints such as `where Length(0, 32)` are checked by the
  compiler wherever it knows the value, so `give(5000)` against
  `Overdose(4000)` doesn't compile, and contracts are checked the same way.
  Values only known while running are checked as the program runs, and every
  type has a known size.
- **Portability:** Croft compiles to C89, needing no additional runtime, so it
  runs wherever a C compiler does, from modern desktops to the ZX Spectrum and
  Atari ST. Ports and adapters make the program the same on every machine, and
  `croft.toml` configures what connects to them.

## Getting started

```croft
context Greeter {
  publish { greet }

  fn greet(): String where Length(0, 12) = {
    "Hello, World"
  }
}

domain Application {
  contexts {
    Greeter
  }

  use { greet } from Greeter

  calls { Console }
} = {
  let output = greet() 

  Console.print(output) // Hello, World
}
```

Save it as `hello.croft`, then:

```sh
cargo install --path crates/croft_cli
croft run hello.croft
```

## Reference

See [docs/reference](./docs/reference) for the language, the `croft` command
and `croft.toml`, and [editors](./editors) for VS Code and Zed.

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
