import { join } from "path";
import { homedir } from "os";
import { mkdir } from "fs/promises";
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
  workspaceDigest: string,
): Promise<string> {
  const configDir = await ensureConfigDirectory();
  const workspaceDir = join(configDir, "workspaces", workspaceDigest);
  await mkdir(workspaceDir, { recursive: true });
  return workspaceDir;
}

export async function getWorkspaceProcessRcPath(
  workspaceDigest: string,
): Promise<string> {
  const workspaceDir = await ensureWorkspaceConfigDirectory(workspaceDigest);
  return join(workspaceDir, "servers.json");
}
