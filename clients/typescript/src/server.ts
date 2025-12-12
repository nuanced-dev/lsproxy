import type { Readable, Writable } from "node:stream";
import WebSocket from "ws";
import type { NuancedLspClient } from "./client.js";
import type { JsonRpcMessage, ServerCommandOptions } from "./types.js";
import { isErr, JsonRpcErrorCode } from "./types.js";

// ---- Types ------------------------------------------------------------------

/**
 * LSP MessageType for window/logMessage notifications.
 */
enum MessageType {
  Error = 1,
  Warning = 2,
  Info = 3,
  Log = 4,
  Debug = 5,
}

// ---- High-level server function ---------------------------------------------

class LspServer {
  private readonly client: NuancedLspClient;
  private readonly workspace: string;
  private readonly opts: ServerCommandOptions;
  private readonly input: Readable;
  private readonly output: Writable;
  private readonly logLevel: MessageType;
  private shutdownReceived = false;
  private isShuttingDown = false;
  private ws: WebSocket | null = null;

  constructor(
    client: NuancedLspClient,
    workspace: string,
    opts: ServerCommandOptions,
    input: Readable,
    output: Writable,
  ) {
    this.client = client;
    this.workspace = workspace;
    this.opts = opts;
    this.input = input;
    this.output = output;
    this.logLevel = opts.debug ? MessageType.Debug : MessageType.Info;
  }

  async run(): Promise<void> {
    try {
      await this.startServer();
      if (this.opts.shared === "up") {
        await this.sendLogMessage(MessageType.Info, "Nuanced LSP started");
        return;
      }
      this.setupSignalHandlers();
      await this.sendLogMessage(MessageType.Info, "Nuanced LSP started");
      await this.processMessages();
    } catch (err) {
      await this.sendLogMessage(
        MessageType.Error,
        `Fatal Nuanced LSP error: ${err}`,
      );
      throw err;
    } finally {
      await this.stopServer();
      if (!this.opts.shared) {
        await this.sendLogMessage(MessageType.Info, "Nuanced LSP stopped");
      } else {
        await this.sendLogMessage(
          MessageType.Info,
          "Nuanced LSP disconnected (shared container left running)",
        );
      }
      process.exit(0);
    }
  }

  private async startServer(): Promise<void> {
    if (this.opts.shared) {
      const statusRes = await this.client.status();
      if (statusRes.ok) {
        await this.sendLogMessage(
          MessageType.Info,
          `Using existing shared container '${this.client.containerName}'`,
        );
        return;
      }
    }

    const res = await this.client.up(this.workspace, {
      proxyImage: this.opts.proxyImage,
      watchdogImage: this.opts.watchdogImage,
      wrapperImage: this.opts.wrapperImage,
      languageContainerVersion: this.opts.languageContainerVersion,
      timeout: this.opts.timeout,
      stream: false,
      ro: this.opts.ro,
      bindHost: this.opts.bindHost,
      debug: this.opts.debug,
      env: this.opts.env,
      envFile: this.opts.envFile,
    });

    if (isErr(res)) {
      await this.sendLogMessage(
        MessageType.Error,
        `Failed to start LSP server container: ${JSON.stringify(res.data)}`,
      );
      throw new Error("Failed to start container");
    }

    await this.waitUntilServerIsHealthy();

    this.ws = await this.client.lsp_ws();
    await this.connectWebSocket();
  }

  private async waitUntilServerIsHealthy(): Promise<void> {
    const timeoutMs = 60000;
    const initialDelayMs = 100;
    const maxDelayMs = 5000;
    const startTime = Date.now();

    let attempt = 0;
    while (Date.now() - startTime < timeoutMs) {
      const result = await this.client.health();

      if (result.ok) {
        if (result.data.status === "ok") {
          return;
        } else if (result.data.status === "not ok") {
          throw new Error("Server is unhealthy");
        }
      }

      const delayMs = Math.min(
        initialDelayMs * Math.pow(2, attempt),
        maxDelayMs,
      );
      await new Promise((resolve) => setTimeout(resolve, delayMs));
      attempt++;
    }

    throw new Error("Server health check timed out after 60s");
  }

  private async connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.ws) {
        reject(new Error("WebSocket not initialized"));
        return;
      }

      this.ws.on("open", () => {
        this.sendLogMessage(MessageType.Debug, "WebSocket connected");
        resolve();
      });

      this.ws.on("error", (err: Error) => {
        this.sendLogMessage(
          MessageType.Error,
          `WebSocket error: ${err.message}`,
        );
        reject(err);
      });

      this.ws.on("close", () => {
        this.sendLogMessage(MessageType.Debug, "WebSocket closed");
      });

      this.ws.on("message", (data: Buffer | string) => {
        try {
          const message = JSON.parse(data.toString()) as JsonRpcMessage;
          // Forward server notifications to stdout
          writeMessage(this.output, message);
        } catch (err) {
          this.sendLogMessage(
            MessageType.Error,
            `Failed to parse WebSocket message: ${err}`,
          );
        }
      });
    });
  }

  private setupSignalHandlers(): void {
    process.on("SIGINT", () => {
      this.stopServer().then(() => process.exit(0));
    });
    process.on("SIGTERM", () => {
      this.stopServer().then(() => process.exit(0));
    });
  }

  private async stopServer(): Promise<void> {
    if (this.isShuttingDown) return;
    this.isShuttingDown = true;

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    if (!this.opts.shared) {
      await this.client.down();
    }
  }

  private async processMessages(): Promise<void> {
    while (true) {
      const message = await readMessage(this.input);

      if (message === null) {
        break;
      }

      if (message.method === "shutdown") {
        await this.handleShutdownRequest(message);
      } else if (message.method === "exit") {
        this.handleExitNotification();
      } else {
        await this.forwardMessage(message);
      }
    }
  }

  private async handleShutdownRequest(message: JsonRpcMessage): Promise<void> {
    await this.sendLogMessage(
      MessageType.Debug,
      "Nuanced LSP shutdown request received",
    );

    this.shutdownReceived = true;
    await this.stopServer();

    const shutdownResponse: JsonRpcMessage = {
      jsonrpc: "2.0",
      id: message.id ?? null,
      result: null,
    };
    await writeMessage(this.output, shutdownResponse);
  }

  private handleExitNotification(): void {
    this.sendLogMessage(
      MessageType.Debug,
      "Nuanced LSP exit notification received",
    );

    const exitCode = this.shutdownReceived ? 0 : 1;
    process.exit(exitCode);
  }

  private async forwardMessage(message: JsonRpcMessage): Promise<void> {
    await this.sendLogMessage(
      MessageType.Debug,
      `Nuanced LSP request: ${JSON.stringify(message)}`,
    );

    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      await this.sendLogMessage(MessageType.Error, "WebSocket not connected");

      if (message.id !== undefined && message.id !== null) {
        const errorResponse: JsonRpcMessage = {
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: JsonRpcErrorCode.InternalError,
            message: "WebSocket not connected",
          },
        };
        await writeMessage(this.output, errorResponse);
      }
      return;
    }

    try {
      this.ws.send(JSON.stringify(message));
    } catch (err) {
      await this.sendLogMessage(
        MessageType.Error,
        `Nuanced LSP forwarding error: ${err}`,
      );

      // Only send error response for requests
      if (message.id !== undefined && message.id !== null) {
        const errorResponse: JsonRpcMessage = {
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: JsonRpcErrorCode.InternalError,
            message: "Failed to send message over WebSocket",
            data: String(err),
          },
        };
        await writeMessage(this.output, errorResponse);
      }
    }
  }

  private async sendLogMessage(
    type: MessageType,
    message: string,
  ): Promise<void> {
    if (type > this.logLevel) return;

    const notification: JsonRpcMessage = {
      jsonrpc: "2.0",
      method: "window/logMessage",
      params: {
        type,
        message,
      },
    };

    await writeMessage(this.output, notification);
  }
}

/**
 * Start an LSP server that manages a container and forwards JSON-RPC requests.
 */
export async function runLspServer(
  client: NuancedLspClient,
  workspace: string,
  opts: ServerCommandOptions,
  input: Readable = process.stdin,
  output: Writable = process.stdout,
): Promise<void> {
  const server = new LspServer(client, workspace, opts, input, output);
  await server.run();
}

// ---- Helper functions -------------------------------------------------------

/**
 * Read a single JSON-RPC message from stdin with Content-Length header parsing.
 * Returns null when stdin is closed.
 */
async function readMessage(input: Readable): Promise<JsonRpcMessage | null> {
  return new Promise((resolve, reject) => {
    let contentLength: number | null = null;
    let buffer = Buffer.alloc(0);
    let headersDone = false;

    const dataHandler = (chunk: Buffer) => {
      buffer = Buffer.concat([buffer, chunk]);

      if (!headersDone) {
        // Try CRLF separator first (LSP standard), then LF
        let separator = "\r\n\r\n";
        let lineSeparator = "\r\n";
        let separatorIndex = buffer.indexOf(separator);

        if (separatorIndex === -1) {
          separator = "\n\n";
          lineSeparator = "\n";
          separatorIndex = buffer.indexOf(separator);
        }

        if (separatorIndex === -1) {
          // Haven't received all headers yet
          return;
        }

        // Extract headers
        const headerSection = buffer
          .subarray(0, separatorIndex)
          .toString("utf8");
        const headers = headerSection.split(lineSeparator);

        for (const header of headers) {
          const len = parseContentLength(header);
          if (len !== null) {
            contentLength = len;
            break;
          }
        }

        if (contentLength === null) {
          // Invalid: no Content-Length header
          cleanup();
          resolve(null);
          return;
        }

        // Move past headers
        buffer = buffer.subarray(separatorIndex + separator.length);
        headersDone = true;
      }

      // Check if we have all the content
      if (headersDone && buffer.length >= contentLength!) {
        // Extract exactly contentLength bytes
        const messageContent = buffer
          .subarray(0, contentLength!)
          .toString("utf8");

        try {
          const message = JSON.parse(messageContent) as JsonRpcMessage;
          cleanup();
          resolve(message);
        } catch (err) {
          // Invalid JSON
          cleanup();
          reject(new Error(`Failed to parse JSON-RPC message: ${err}`));
        }
      }
    };

    const endHandler = () => {
      cleanup();
      resolve(null);
    };

    const errorHandler = (err: Error) => {
      cleanup();
      reject(err);
    };

    const cleanup = () => {
      input.off("data", dataHandler);
      input.off("end", endHandler);
      input.off("error", errorHandler);
    };

    input.on("data", dataHandler);
    input.on("end", endHandler);
    input.on("error", errorHandler);
  });
}

/**
 * Parse Content-Length header from LSP message.
 */
function parseContentLength(header: string): number | null {
  const match = /^Content-Length:\s*(\d+)\s*$/i.exec(header.trim());
  return match ? parseInt(match[1], 10) : null;
}

/**
 * Write a JSON-RPC message to stdout with Content-Length header.
 */
function writeMessage(
  output: Writable,
  message: JsonRpcMessage,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const content = JSON.stringify(message);
    const contentLength = Buffer.byteLength(content, "utf8");
    const header = `Content-Length: ${contentLength}\r\n\r\n`;
    const full = header + content;

    output.write(full, (err) => {
      if (err) reject(err);
      else resolve();
    });
  });
}
