import { MutagenError } from "@nuanced-dev/mutagen";
import { spawnMutagen } from "../util/mutagen.js";

export async function mutagenCommand(args: string[]) {
  try {
    const o = await spawnMutagen(args);
    console.log(o.stdout);
  } catch (e) {
    const me = e as MutagenError;
    console.error(me.stderr);
    process.exitCode = me.exitCode;
  }
}
