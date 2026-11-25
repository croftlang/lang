# tree-sitter-croft

A [Tree-sitter](https://tree-sitter.github.io) grammar for Croft. Zed uses it
through `editors/zed`, and Neovim and Helix can load it too.

`grammar.js` follows the parser in `crates/croft_syntax/src/parser`. `src/` is
generated from it and is committed, because editors build the grammar from
`src/parser.c`.

## Working on it

```sh
cd editors/tree-sitter-croft
npx tree-sitter-cli generate     # rebuild src/ after changing grammar.js
npx tree-sitter-cli test         # run test/corpus
npx tree-sitter-cli parse path/to/file.croft
```

Run `generate` and `test` after every change to `grammar.js`, and commit the
new `src/`. When a test's tree changes on purpose, `test --update` rewrites
it; read the new tree before committing it.

## Where it differs from the compiler

The compiler ends an expression at a line break, except before `.` and `?`.
Tree-sitter treats a line break as a space, so a line that starts with a
binary operator or `(` carries on the line above. The compiler reads that
as two statements. Code is rarely written that way.

In an `if` or `match` condition, the compiler reads `{` after a name as the
start of the block. The grammar tries both a record and a block and keeps
the one that parses.

## Highlight queries

`queries/highlights.scm` is written for editors where later patterns win,
as in Zed and Neovim. `editors/zed/languages/croft/highlights.scm` is a copy;
change both together.
