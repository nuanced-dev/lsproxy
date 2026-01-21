#!/usr/bin/env node

import { Command } from "commander";

const program = new Command();

program
  .name("managed-lsp")
  .description("Managed LSP CLI for containerized LSP services")
  .version("0.3.19");

const service = program.command("service").description("Service management");

service
  .command("create")
  .description("Create service snapshot")
  .action(async () => {
    const { serviceCreate: serviceSetup } =
      await import("./commands/service-create");
    serviceSetup().catch(console.error);
  });

service
  .command("status")
  .description("Show service status")
  .action(async () => {
    const { serviceStatus } = await import("./commands/service-status");
    serviceStatus().catch(console.error);
  });

service
  .command("delete")
  .description("Delete service snapshot")
  .action(async () => {
    const { serviceDelete } = await import("./commands/service-delete");
    serviceDelete().catch(console.error);
  });

const workspace = program
  .command("workspace")
  .description("Workspace management");

workspace
  .command("server")
  .description("Run LSP server for a workspace")
  .argument("<directory>", "Workspace directory")
  .action(async (directory: string) => {
    const { workspaceServer: workspaceLsp } =
      await import("./commands/workspace-server");
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
  .command("delete")
  .description("Delete a workspace")
  .argument("<directory>", "Workspace directory")
  .action(async (directory: string) => {
    const { workspaceDelete: workspaceStop } =
      await import("./commands/workspace-delete");
    workspaceStop(directory).catch(console.error);
  });

program.parse();
