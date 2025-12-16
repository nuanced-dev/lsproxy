#!/usr/bin/env node

import { Instance, MorphCloudClient } from 'morphcloud';
import { LABEL_NUANCED_ROLE, LABEL_NUANCED_WORKSPACE_DIGEST, NUANCED_ROLE_WORKSPACE } from './util/constants';
import { findInstance } from './util/morphcloud';
import { MutagenClient } from './util/mutagen';
import { realpathSync } from 'fs';
import { getGloballyUniqueDigest } from './util/digest';

async function main() {
    const client = new MorphCloudClient();

    if (process.argv.length !== 3) {
        console.error(`Usage: ${process.argv[1]} WORKSPACE_DIR`);
        process.exit(1);
    }
    const workspaceDir = realpathSync(process.argv[2]);
    const workspaceDigest = getGloballyUniqueDigest(workspaceDir);
    const metadata = {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_WORKSPACE,
        [LABEL_NUANCED_WORKSPACE_DIGEST]: workspaceDigest,
    };

    let workspaceInstance: Instance | undefined;
    try {
        workspaceInstance = await findInstance(client, { metadata });

        if (workspaceInstance) {
            const mutagen = new MutagenClient(workspaceInstance);
            await mutagen.stopSync().catch((_) => {
                    console.error(`Error: Cannot remove sync ${workspaceDigest}`);
            });
            await workspaceInstance.stop().catch((e) => {
                console.error(`Error: Cannot stop instance ${workspaceInstance!.id}`);
            });
        }
    } catch (error) {
        console.error('Error:', error);
        process.exitCode = 1;
    } finally {
    }
}

main().catch(console.error);
