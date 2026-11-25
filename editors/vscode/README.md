# b for VS Code

Syntax highlighting for `.b` files and ` ```b ` code blocks in Markdown,
and, through `b lsp`, errors as you type, hover and go to definition.

The language server is the `b` command, so install it first:

```sh
cargo install --path crates/b_cli
```

If `b` is not on your PATH, set `b.server.path` to it in your settings.

The grammar is `syntaxes/b.tmLanguage.json`, a TextMate grammar. Editors that
read TextMate grammars (Sublime Text, JetBrains IDEs through TextMate bundles,
and Shiki on the web) can use the same file.

## Install from this folder

Install the language client it uses, link the folder into VS Code's
extensions, and reload the window:

```sh
(cd editors/vscode && npm install)
ln -s "$PWD/editors/vscode" ~/.vscode/extensions/briward.b-language-0.1.0
```

Or build a `.vsix` and install that:

```sh
cd editors/vscode
npm install
npx @vscode/vsce package
code --install-extension b-language-0.1.0.vsix
```

## Keeping it in step with the lexer

The keywords come from `crates/b_syntax/src/keyword.rs`. Words that keyword
list leaves out, such as `from`, `for`, `on`, `when`, `include` and
`contexts`, are only highlighted where the parser expects them. `event` and
`error` are keywords only when a name follows, as in `event PlayerMoved`.
Anywhere else they are names, and so is any word after `.` or before `:`.
When a keyword is added to `keyword.rs`, add it to the `keywords` rule in the
grammar too.
