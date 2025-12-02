"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
let client;
function startClient() {
    const config = vscode.workspace.getConfiguration("nuancedLsp");
    const command = config.get("command", "nuanced-lsp");
    const proxyImage = config.get("proxyImage", "");
    const watchdogImage = config.get("watchdogImage", "");
    const wrapperImage = config.get("wrapperImage", "");
    const debug = config.get("debug", false);
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage("Nuanced LSP: No workspace folder found. Please open a folder to use this extension.");
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
    const serverOptions = {
        command,
        args,
    };
    const clientOptions = {
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
    client = new node_1.LanguageClient("nuancedLsp", "Nuanced LSP", serverOptions, clientOptions);
    client.start();
}
async function restartClient() {
    if (client) {
        await client.stop();
    }
    startClient();
    vscode.window.showInformationMessage("Nuanced LSP: Server restarted");
}
function activate(context) {
    startClient();
    context.subscriptions.push(vscode.commands.registerCommand("nuancedLsp.restart", restartClient));
    context.subscriptions.push({
        dispose: () => {
            if (client) {
                client.stop();
            }
        },
    });
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map