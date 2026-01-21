import {
  ExecOptions,
  Instance,
  InstanceBootOptions,
  InstanceListOptions,
  InstanceStartOptions,
  InstanceStatus,
  MorphCloudClient,
  Snapshot,
} from "morphcloud";

export async function findSnapshot(
  client: MorphCloudClient,
  opts?: {
    metadata?: Record<string, string>;
  },
): Promise<Snapshot | undefined> {
  const snapshots = await client.snapshots.list(opts);
  if (snapshots.length > 1) {
    throw new Error("Multiple matching snapshots");
  }
  return snapshots[0];
}

export async function bootInstance(
  client: MorphCloudClient,
  snapshot: Snapshot,
  opts?: {
    vcpus?: number;
    memory?: number;
    diskSize?: number;
    metadata?: Record<string, string>;
  },
): Promise<Instance> {
  const bootOpts: InstanceBootOptions = {
    ttlSeconds: 3600,
    ...opts,
    snapshotId: snapshot.id,
  };
  const instance = await client.instances.boot(bootOpts);
  await instance.waitUntilReady(30);
  return instance;
}

export async function startInstance(
  client: MorphCloudClient,
  snapshot: Snapshot,
  opts?: {
    metadata?: Record<string, string>;
  },
): Promise<Instance> {
  const startOpts: InstanceStartOptions = {
    ttlSeconds: 3600,
    ...opts,
    snapshotId: snapshot.id,
  };
  const instance = await client.instances.start(startOpts);
  await instance.waitUntilReady(30);
  return instance;
}

export async function findInstance(
  client: MorphCloudClient,
  opts?: {
    metadata?: Record<string, string>;
    ensureReady?: boolean;
  },
): Promise<Instance> {
  const listOpts: InstanceListOptions = {
    ...opts,
  };
  const instances = await client.instances.list(listOpts);
  if (instances.length > 1) {
    throw new Error("Multiple matching instances");
  }
  const instance = instances[0];
  if (instance) {
    switch (instance.status) {
      case InstanceStatus.ERROR:
        throw new Error(`instance ${instance.id} in error state`);
      case InstanceStatus.PAUSED:
        if (opts?.ensureReady) {
          await instance.resume();
          await instance.waitUntilReady(30);
        }
        break;
      case InstanceStatus.PENDING:
        if (opts?.ensureReady) {
          await instance.waitUntilReady(30);
        }
        break;
      case InstanceStatus.SAVING:
        if (opts?.ensureReady) {
          await instance.waitUntilReady(30);
        }
        break;
    }
  }
  return instance;
}

export async function listInstances(
  client: MorphCloudClient,
  opts?: {
    metadata?: Record<string, string>;
  },
): Promise<Instance[]> {
  const listOpts: InstanceListOptions = {
    ...opts,
  };
  return await client.instances.list(listOpts);
}

export function formatInstanceStatus(status: InstanceStatus): string {
  switch (status) {
    case InstanceStatus.ERROR:
      return "ERROR";
    case InstanceStatus.PENDING:
      return "PENDING";
    case InstanceStatus.READY:
      return "READY";
    case InstanceStatus.PAUSED:
      return "PAUSED";
    case InstanceStatus.SAVING:
      return "SAVING";
  }
}

export async function execOrThrow(
  instance: Instance,
  cmd: string,
  opts?: {
    verbose?: boolean;
    prefix?: string;
  },
): Promise<string> {
  const { verbose = false, prefix = "" } = opts || {};
  function logger(inner: (line: string) => void) {
    return (content: string) =>
      content.split("\n").forEach((line: string) => inner(`${prefix}${line}`));
  }
  const execOpts: ExecOptions = {};
  if (verbose) {
    execOpts.onStdout = logger(console.log);
    execOpts.onStderr = logger(console.error);
  }
  try {
    const response = await instance.exec(cmd, execOpts);
    return response.stdout;
  } catch (e) {
    throw new Error(`Exec failed: ${e}`);
  }
}
