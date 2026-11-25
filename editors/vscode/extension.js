// Starts `b lsp` for .b files, for errors as you type, hover and go to definition.

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace.getConfiguration("b").get("server.path") || "b";
  client = new LanguageClient(
    "b",
    "b",
    { command, args: ["lsp"] },
    { documentSelector: [{ scheme: "file", language: "b" }] },
  );
  client.start().catch((error) => {
    vscode.window.showErrorMessage(
      `Could not start the b language server with \`${command} lsp\`: ${error.message}. ` +
        "Install b with `cargo install --path crates/b_cli`, or set `b.server.path`.",
    );
  });
  context.subscriptions.push({ dispose: () => client && client.stop() });
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
