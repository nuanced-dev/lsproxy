import { spawnMutagen } from "../util/mutagen.js";

export async function mutagenCommand(args: string[]) {
  await spawnMutagen(args);
}
