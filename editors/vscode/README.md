# Croft for VS Code

Support for [Croft](https://croftlang.com), the domain-driven language that compiles to
portable C89:

- highlighting for `.croft` and `.cro` files, and for ` ```croft ` blocks in Markdown
- errors as you type, hover, go to definition and completion, through `croft lsp`
- the Croft mark on Croft files, unless your file icon theme has its own

## Before you start

The language server is the `croft` command, so [install Croft](https://croftlang.com/install)
first. If `croft` isn't on your `PATH`, set `croft.server.path` to it in your settings.

New to Croft? Start with [Hello world](https://croftlang.com/learn/hello-world).

## Developing

Link this folder into VS Code's extensions and reload the window:

```sh
(cd editors/vscode && npm install)
ln -s "$PWD/editors/vscode" ~/.vscode/extensions/croftlang.croft-dev
```

Or build a `.vsix` and install that:

```sh
cd editors/vscode
npm install
npx @vscode/vsce package
code --install-extension croft-*.vsix
```

Pushing a version tag publishes it to the VS Code Marketplace and Open VSX, from
`.github/workflows/release.yml`.

The grammar is `syntaxes/croft.tmLanguage.json`, a TextMate grammar. Editors that read
TextMate grammars (Sublime Text, JetBrains IDEs through TextMate bundles, and Shiki on the
web) can use the same file.

### Keeping it in step with the lexer

The keywords come from `crates/croft_syntax/src/keyword.rs`. Words that keyword list leaves
out, such as `from`, `for`, `on`, `when`, `include` and `contexts`, are only highlighted where
the parser expects them. `event` and `error` are keywords only when a name follows, as in
`event PlayerMoved`. Anywhere else they are names, and so is any word after `.` or before `:`.
When a keyword is added to `keyword.rs`, add it to the `keywords` rule in the grammar too.
