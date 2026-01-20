import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

const DEFAULT_COMMAND_CONFIG = ["nuanced-lsp", "server", "--host-port", "0"];

let client: LanguageClient | undefined;

function startClient() {
  const config = vscode.workspace.getConfiguration("nuancedLsp");
  const commandConfig = config.get<string | string[] | undefined>("command");
  const envConfig = config.get<Record<string, string>>("env", {});

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage(
      "Nuanced LSP: No workspace folder found. Please open a folder to use this extension.",
    );
    return;
  }

  const workspacePath = workspaceFolder.uri.fsPath;

  let command: string;
  let args: string[];

  if (Array.isArray(commandConfig)) {
    [command, ...args] = commandConfig;
  } else if (typeof commandConfig === "string") {
    [command, ...args] = [commandConfig];
  } else {
    [command, ...args] = DEFAULT_COMMAND_CONFIG;
  }

  args.push(workspacePath);

  const serverOptions: ServerOptions = {
    command,
    args,
    options: {
      env: { ...process.env, ...envConfig },
    },
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
