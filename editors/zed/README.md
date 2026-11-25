# b for Zed

Syntax highlighting, the outline panel, bracket matching, indentation and
text objects for `.b` files, and, through `b lsp`, errors as you type,
hover, go to definition and completion. The grammar is `editors/tree-sitter-b`.

The language server is the `b` command, which the extension finds on your
PATH, so install it first:

```sh
cargo install --path crates/b_cli
```

`src/lib.rs` starts it. Zed compiles that to WebAssembly when it installs
the extension, which needs Rust installed through rustup.

## Installing it

Zed builds the grammar from git, at the commit named by `rev` in
`extension.toml`, so the grammar must be committed before Zed can use it.

1. Commit `editors/tree-sitter-b`, and push it if `repository` points at
   GitHub.
2. Set `rev` in `extension.toml` to that commit's SHA. A branch name such as
   `main` works too, but a SHA keeps the build fixed.
3. In Zed, run `zed: install dev extension` and choose `editors/zed`.

To try changes before pushing, point `repository` at your clone with a
`file://` URL, such as `file:///Users/you/Code/b`, and set `rev` to a local
commit.

If the grammar fails to build, `zed: open log` shows why.

## Keeping it in step with the grammar

`languages/b/highlights.scm` is a copy of
`editors/tree-sitter-b/queries/highlights.scm`; change both together. The
other queries name grammar nodes, so check them after renaming a node in
`grammar.js`.
