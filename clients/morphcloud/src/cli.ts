#!/usr/bin/env node

import { Command } from "commander";

const program = new Command();

program
  .name("managed-lsp")
  .description("Managed LSP CLI for containerized LSP services")
  .version("0.3.19");

const service = program.command("service").description("Service management");

service
  .command("setup")
  .description("Set up service snapshots")
  .action(async () => {
    const { serviceSetup } = await import("./commands/service-setup");
    serviceSetup().catch(console.error);
  });

service
  .command("status")
  .description("Show service status")
  .action(async () => {
    const { serviceStatus } = await import("./commands/service-status");
    serviceStatus().catch(console.error);
  });

const workspace = program
  .command("workspace")
  .description("Workspace management");

workspace
  .command("lsp")
  .description("Run LSP server for a workspace")
  .argument("<directory>", "Workspace directory")
  .action(async (directory: string) => {
    const { workspaceLsp } = await import("./commands/workspace-lsp");
    workspaceLsp(directory).catch(console.error);
  });

workspace
  .command("status")
  .description("Show workspace status")
  .argument("<directory>", "Workspace directory")
  .action(async (directory: string) => {
    const { workspaceStatus } = await import("./commands/workspace-status");
    workspaceStatus(directory).catch(console.error);
  });

workspace
  .command("stop")
  .description("Stop a workspace")
  .argument("<directory>", "Workspace directory")
  .action(async (directory: string) => {
    const { workspaceStop } = await import("./commands/workspace-stop");
    workspaceStop(directory).catch(console.error);
  });

program.parse();
