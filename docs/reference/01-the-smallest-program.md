# The smallest program

One context with one function, and a domain that calls it and prints the answer through the built-in Console port. It needs no croft.toml: `croft run hello.croft` runs it on the `desktop` target.

```croft
context Hello {
    publish { add }

    fn add(a: Int, b: Int): Int = a + b
}

domain Program {
    contexts { Hello }
    use { add } from Hello
    calls { Console }
} = Console.print(add(2, 3).to_string())   // prints 5
```

Source files end in `.croft`, or in `.cro` on machines whose file names are limited to 8.3; a project can mix the two, and the compiler reads either in capitals too, as in `CARGO.CRO`. Without a croft.toml the program runs on the `desktop` target; add one to name adapters, add targets or set the program's name (see the project file). The `croft` command checks, compiles, runs, measures and lays out a project, a folder or a single file. `croft lsp` is the language server the editor extensions use: errors as you type, hover, go to definition, completion and formatting. The logistics fixture, a small cargo shipping system after the one in Eric Evans' _Domain-Driven Design_, is a larger program to try it on.

```sh
cargo install --path crates/croft_cli

croft check .                        # compiler check
croft compile .                      # check and compile to C
croft run .                          # check, compile and run the C
croft size . --target spectrum_48k   # memory each type and function takes
croft fmt .                          # lay out the source files in the Croft style
croft lsp                            # the language server, for editors

croft check tests/fixtures/logistics
croft compile tests/fixtures/logistics --target desktop
```
