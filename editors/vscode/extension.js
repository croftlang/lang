// Starts `croft lsp` for .croft and .cro files, for errors as you type, hover and go to definition.

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace.getConfiguration("croft").get("server.path") || "croft";
  client = new LanguageClient(
    "croft",
    "Croft",
    { command, args: ["lsp"] },
    { documentSelector: [{ scheme: "file", language: "croft" }] },
  );
  client.start().catch((error) => {
    vscode.window.showErrorMessage(
      `Could not start the Croft language server with \`${command} lsp\`: ${error.message}. ` +
        "Install Croft with `cargo install --path crates/croft_cli`, or set `croft.server.path`.",
    );
  });
  context.subscriptions.push({ dispose: () => client && client.stop() });
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
