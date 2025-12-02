import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

function startClient() {
  const config = vscode.workspace.getConfiguration("nuancedLsp");
  const command = config.get<string>("command", "nuanced-lsp");
  const proxyImage = config.get<string>("proxyImage", "");
  const watchdogImage = config.get<string>("watchdogImage", "");
  const wrapperImage = config.get<string>("wrapperImage", "");
  const debug = config.get<boolean>("debug", false);

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage(
      "Nuanced LSP: No workspace folder found. Please open a folder to use this extension.",
    );
    return;
  }

  const workspacePath = workspaceFolder.uri.fsPath;

  const args = ["server", "--host-port", "0"];
  if (proxyImage) {
    args.push("--proxy-image", proxyImage);
  }
  if (watchdogImage) {
    args.push("--watchdog-image", watchdogImage);
  }
  if (wrapperImage) {
    args.push("--wrapper-image", wrapperImage);
  }
  if (debug) {
    args.push("--debug");
  }
  args.push(workspacePath);

  const serverOptions: ServerOptions = {
    command,
    args,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "python" },
      { scheme: "file", language: "typescript" },
      { scheme: "file", language: "javascript" },
      { scheme: "file", language: "rust" },
      { scheme: "file", language: "cpp" },
      { scheme: "file", language: "csharp" },
      { scheme: "file", language: "java" },
      { scheme: "file", language: "go" },
      { scheme: "file", language: "php" },
      { scheme: "file", language: "ruby" },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*"),
    },
  };

  client = new LanguageClient(
    "nuancedLsp",
    "Nuanced LSP",
    serverOptions,
    clientOptions,
  );

  client.start();
}

async function restartClient() {
  if (client) {
    await client.stop();
  }
  startClient();
  vscode.window.showInformationMessage("Nuanced LSP: Server restarted");
}

export function activate(context: vscode.ExtensionContext) {
  startClient();

  context.subscriptions.push(
    vscode.commands.registerCommand("nuancedLsp.restart", restartClient),
  );

  context.subscriptions.push({
    dispose: () => {
      if (client) {
        client.stop();
      }
    },
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
