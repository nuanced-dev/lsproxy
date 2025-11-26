import path from "node:path";

import { ClientRunner } from "./runner.js";
import { relToWorkspace } from "./fixtures.js";
import { isGeneratedPath } from "./constants.js";

const filteredFilesCache = new Map<string, string[]>();
const definitionsCache = new Map<string, Map<string, any[]>>();

function workspaceKey(runner: ClientRunner): string {
  return path.resolve(runner.workspace);
}

function allowedExtensionsFor(langKey: string): string[] | null {
  const lower = langKey.toLowerCase();
  if (lower.startsWith("csharp")) return [".cs"];
  if (lower.startsWith("cpp"))
    return [".c", ".cc", ".cpp", ".cxx", ".h", ".hpp", ".hxx"];
  if (lower.startsWith("go")) return [".go"];
  if (lower.startsWith("java")) return [".java"];
  if (lower.startsWith("php")) return [".php"];
  if (lower.startsWith("python")) return [".py"];
  if (lower.startsWith("ruby")) return [".rb", ".rbi"];
  if (lower.startsWith("rust")) return [".rs"];
  if (lower === "ts") return [".ts", ".tsx"];
  if (lower === "js") return [".js", ".jsx", ".ts", ".tsx"];
  return null;
}

function isSupportedSourceFile(langKey: string, filePath: string): boolean {
  const exts = allowedExtensionsFor(langKey);
  if (!exts) return true;
  const ext = path.extname(filePath).toLowerCase();
  return ext.length > 0 && exts.includes(ext);
}

export async function workspaceFiles(runner: ClientRunner): Promise<string[]> {
  const key = workspaceKey(runner);
  runner.ensureUp();
  if (filteredFilesCache.has(key)) {
    return filteredFilesCache.get(key)!;
  }

  const files = await runner.listFiles();
  const filtered = files.filter((file) => {
    const rel = relToWorkspace(runner.workspace, file);
    return (
      !isGeneratedPath(rel) && isSupportedSourceFile(runner.language.key, rel)
    );
  });
  filteredFilesCache.set(key, filtered);
  return filtered;
}

export async function workspaceDefinitions(
  runner: ClientRunner,
  files: string[],
): Promise<Map<string, any[]>> {
  runner.ensureUp();
  const wsKey = workspaceKey(runner);
  const defsForWorkspace =
    definitionsCache.get(wsKey) ?? new Map<string, any[]>();

  for (const file of files) {
    if (!defsForWorkspace.has(file)) {
      const payload = await runner.definitionsInFile(file);
      const data = Array.isArray(payload)
        ? payload
        : Array.isArray(payload?.definitions)
          ? payload.definitions
          : [];
      defsForWorkspace.set(file, data);
    }
  }

  definitionsCache.set(wsKey, defsForWorkspace);
  return defsForWorkspace;
}

export function resetCaches(): void {
  filteredFilesCache.clear();
  definitionsCache.clear();
}
