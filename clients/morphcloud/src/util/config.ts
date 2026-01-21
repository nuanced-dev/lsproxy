import { join } from "path";
import { homedir } from "os";
import { mkdir } from "fs/promises";
import { createHash } from "crypto";
import { Instance } from "morphcloud";

export async function ensureConfigDirectory(): Promise<string> {
  const configDir = join(homedir(), ".nuanced");
  await mkdir(configDir, { recursive: true });
  return configDir;
}

export async function ensureInstanceConfigDirectory(
  instance: Instance,
): Promise<string> {
  const configDir = await ensureConfigDirectory();
  const instanceDir = join(configDir, "instances", instance.id);
  await mkdir(instanceDir, { recursive: true });
  return instanceDir;
}

export async function ensureWorkspaceConfigDirectory(
  workspacePath: string,
): Promise<string> {
  const configDir = await ensureConfigDirectory();
  const hash = createHash("sha256")
    .update(workspacePath)
    .digest("hex")
    .substring(0, 12);
  const workspaceDir = join(configDir, "workspaces", hash);
  await mkdir(workspaceDir, { recursive: true });
  return workspaceDir;
}

export async function getWorkspaceProcessRcPath(
  workspacePath: string,
): Promise<string> {
  const workspaceDir = await ensureWorkspaceConfigDirectory(workspacePath);
  return join(workspaceDir, "servers.json");
}
