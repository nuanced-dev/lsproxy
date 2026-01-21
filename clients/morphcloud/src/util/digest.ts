import { createHash } from "crypto";
import { machineIdSync } from "node-machine-id";

const MACHINE_ID = machineIdSync();

export function getLocallyUniqueDigest(value: string): string {
  const hash = createHash("sha256").update(value).digest("hex");
  return hash.substring(0, 12);
}

export function getGloballyUniqueDigest(value: string): string {
  const hash = createHash("sha256")
    .update(MACHINE_ID)
    .update("-")
    .update(value)
    .digest("hex");
  return hash.substring(0, 12);
}
