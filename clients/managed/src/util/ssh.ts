import { rm, writeFile } from "fs/promises";
import { Instance } from "morphcloud";
import { NodeSSH } from "node-ssh";
import { join } from "path";
import { ClientChannel } from "ssh2";
import stream from 'stream';
import { ensureInstanceConfigDirectory } from "./config";

const MORPH_SSH_HOST = "ssh.cloud.morph.so";

export interface SshConfig {
    user: string;
    host: string;
    configPath: string;
};

export async function getSshConfig(instance: Instance): Promise<SshConfig> {
    const configPath = await writeSshConfig(instance);
    return {
        user: instance.id,
        host: MORPH_SSH_HOST,
        configPath,
    };
}

async function writeSshConfig(instance: Instance): Promise <string> {
    const instanceConfigDir = await ensureInstanceConfigDirectory(instance);

    const sshKey = await instance.sshKey();
    const sshKeyPath = join(instanceConfigDir, "ssh_key");
    await writeFile(sshKeyPath, sshKey.private_key, { mode: 0o600 });

    const sshConfigPath = join(instanceConfigDir, "ssh_config");

    const sshConfig = `# Generated file
HostName ${MORPH_SSH_HOST}
User ${instance.id}
IdentityFile ${sshKeyPath}
IdentitiesOnly yes
IdentityAgent none
StrictHostKeyChecking no
WarnWeakCrypto no
UserKnownHostsFile /dev/null
        `;

    await writeFile(sshConfigPath, sshConfig);
    return sshConfigPath;
}

export async function removeSshConfig(instance: Instance): Promise <void> {
    const instanceConfigDir = await ensureInstanceConfigDirectory(instance);
    try {
        await rm(instanceConfigDir, { recursive: true, force: true });
    } catch(e) {
        // Ignore errors during cleanup
    }
}

export function sshExec(ssh: NodeSSH, command: string, args: string[], opts?: {
    stdin?: stream.Readable,
    stdout?: stream.Writable,
    stderr?: stream.Writable,
}): Promise<ClientChannel> {
    return new Promise(async (resolve, reject) => {
        try {
            await ssh.exec(command, args, {
                stdin: opts?.stdin,
                stream: "both",
                onChannel: (stream) => {
                    if (opts?.stdout) {
                        stream.stdout.pipe(opts.stdout);
                    }
                    if (opts?.stderr) {
                        stream.stderr.pipe(opts.stderr);
                    }
                    resolve(stream);
                }
            });
        } catch (e) {
            reject(e);
        }
    });
}
