# Croft for Zed

Support for [Croft](https://croftlang.com) in Zed: highlighting, the outline panel, bracket
matching, indentation and text objects for `.croft` and `.cro` files, and, through `croft lsp`,
errors as you type, hover, go to definition and completion.

Install it from Zed's extensions (`zed: extensions`, then search for Croft). The language
server is the `croft` command, which the extension finds on your `PATH`, so
[install Croft](https://croftlang.com/install) first.

## Developing

`src/lib.rs` starts the language server. Zed compiles it to WebAssembly when it installs the
extension, which needs Rust installed through rustup. The grammar is
`editors/tree-sitter-croft`.

Zed builds the grammar from git, at the commit named by `rev` in `extension.toml`, so the
grammar must be committed before Zed can use it.

1. Commit `editors/tree-sitter-croft`, and push it if `repository` points at GitHub.
2. Set `rev` in `extension.toml` to that commit's SHA. A branch name such as `main` works
   too, but a SHA keeps the build fixed.
3. In Zed, run `zed: install dev extension` and choose `editors/zed`.

To try changes before pushing, point `repository` at your clone with a `file://` URL, such as
`file:///Users/you/Code/lang`, and set `rev` to a local commit.

If the grammar fails to build, `zed: open log` shows why.

To release a new version, raise `version` in `extension.toml` and open a pull request on
[zed-industries/extensions](https://github.com/zed-industries/extensions) that moves the
`extensions/croft` submodule to the new commit and sets the same `version` in
`extensions.toml`.

## Keeping it in step with the grammar

`languages/croft/highlights.scm` is a copy of
`editors/tree-sitter-croft/queries/highlights.scm`; change both together. The
other queries name grammar nodes, so check them after renaming a node in
`grammar.js`.
