// Vitalis VS Code Extension — LSP Client
//
// Launches `vtc lsp` as the language server process and connects via stdio.

const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
const path = require("path");

let client;

function activate(context) {
  const serverCommand = process.platform === "win32" ? "vtc.exe" : "vtc";

  const serverOptions = {
    run: { command: serverCommand, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command: serverCommand, args: ["lsp"], transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "vitalis" }],
  };

  client = new LanguageClient("vitalis", "Vitalis Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  if (client) return client.stop();
}

module.exports = { activate, deactivate };
