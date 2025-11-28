import type { Readable, Writable } from "node:stream";
import type { NuancedLspClient } from "./client.js";
import type {
  JsonRpcMessage,
  JsonRpcRequest,
  JsonRpcResponse,
  ServerCommandOptions,
} from "./types.js";
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
    await this.startServer();
    this.setupSignalHandlers();
    await this.sendLogMessage(MessageType.Info, "Nuanced LSP started");

    try {
      await this.processMessages();
    } catch (err) {
      await this.sendLogMessage(
        MessageType.Error,
        `Fatal Nuanced LSP error: ${err}`,
      );
      throw err;
    } finally {
      await this.stopServer();
      await this.sendLogMessage(MessageType.Info, "Nuanced LSP stopped");
    }
  }

  private async startServer(): Promise<void> {
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

    await this.client.down();
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

  private async handleShutdownRequest(message: JsonRpcRequest): Promise<void> {
    await this.sendLogMessage(
      MessageType.Debug,
      "Nuanced LSP shutdown request received",
    );

    this.shutdownReceived = true;
    await this.stopServer();

    const shutdownResponse: JsonRpcResponse = {
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

  private async forwardMessage(message: JsonRpcRequest): Promise<void> {
    await this.sendLogMessage(
      MessageType.Debug,
      `Nuanced LSP request: ${JSON.stringify(message)}`,
    );

    try {
      // Determine if this is a request (has id) or notification (no id)
      const isRequest = message.id !== undefined;
      const result = isRequest
        ? await this.client.request(message.method, message.params, undefined)
        : await this.client.notify(message.method, message.params, undefined);

      if (result.ok) {
        await this.sendLogMessage(
          MessageType.Debug,
          `Nuanced LSP result reponse: ${JSON.stringify(result.data)}`,
        );

        // Only send response for requests (notifications don't get responses)
        if (isRequest) {
          const response: JsonRpcResponse = {
            jsonrpc: "2.0",
            id: message.id!,
            result: result.data,
          };
          await writeMessage(this.output, response);
        }
      } else {
        await this.sendLogMessage(
          MessageType.Error,
          `Nuanced LSP error response: ${JSON.stringify(result.data)}`,
        );

        // Only send error response for requests
        if (isRequest) {
          // If error is a JsonRpcError (from 500 response), use it directly
          // Otherwise create a generic error
          let error: {
            code: number;
            message: string;
            data?: any;
          };
          if (
            typeof result.data.error === "object" &&
            result.data.error !== null &&
            "code" in result.data.error &&
            "message" in result.data.error
          ) {
            error = result.data.error as {
              code: number;
              message: string;
              data?: any;
            };
          } else {
            error = {
              code: result.data.status_code ?? JsonRpcErrorCode.InternalError,
              message: "LSP forwarding error",
              data: result.data.error,
            };
          }

          const errorResponse: JsonRpcResponse = {
            jsonrpc: "2.0",
            id: message.id!,
            error,
          };
          await writeMessage(this.output, errorResponse);
        }
      }
    } catch (err) {
      // Only send error response for requests
      if (message.id !== undefined) {
        const errorResponse: JsonRpcResponse = {
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: JsonRpcErrorCode.InternalError,
            message: "Internal error",
            data: String(err),
          },
        };
        await writeMessage(this.output, errorResponse);
      }

      await this.sendLogMessage(
        MessageType.Error,
        `Nuanced LSP forwarding error: ${err}`,
      );
    }
  }

  private async sendLogMessage(
    type: MessageType,
    message: string,
  ): Promise<void> {
    if (type > this.logLevel) return;

    const notification: JsonRpcRequest = {
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
async function readMessage(input: Readable): Promise<JsonRpcRequest | null> {
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
          const message = JSON.parse(messageContent) as JsonRpcRequest;
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
