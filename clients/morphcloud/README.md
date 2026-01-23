# Nuanced LSP Morph Cloud Client

This is a client to run Nuanced LSP remotely on [Morph Cloud](https://cloud.morph.so).

_Note that this client is experimental and not considered stable!_

**Current features:**

- Run Nuanced LSP instances for a workspace remotely in the Morph Cloud.
- Sync files from the local workspace to the remote instance.
- Resume a previous LSP instance for the same workspace for fast startup.

**Known issues:**

- Mutagen easily gets into error states, where resuming syncs hangs or setting up syncs fails. Often the only way to recover is removing the mutagen data directory (`~/.nuanced/mutagen`).
- File syncs are sometimes not properly resumed, in which case the workspace instance needs to be recreated.
- Morph Cloud retains snapshots for instances that have been deleted. These snapshots do not inherit the metadata of the instance and we currently cannot clean them up.

**Future possibilities:**

- Use instances from other branches of the same repo to improve cold start time. For example, if an instance is available for the main branch, feature branches could start from that and reuse existing state as much as possible.
- Support creating multiple, named, service snapshots with different resource or languages preloaded.

## Requirements

- Recent Node.js version installed
- A [Morph Cloud](https://cloud.morph.so) account, and an API token in the `MORPH_API_KEY` environment variable.

## Quick start

**Create the service snapshot:**

```bash
nuanced-lsp-morphcloud service create
```

This creates the base snapshot from which the workspace instances are spawned.

**Check service status:**

```bash
nuanced-lsp-morphcloud service status
```

**Delete service snapshot:**

```bash
nuanced-lsp-morphcloud service delete
```

This can be useful if you want to recreate the service snapshot. The old snapshot must be removed first.

**Start a remove LSP server for a workspace:**

```bash
nuanced-lsp-morphcloud workspace server /path/to/workspace
```

**Check workspace status:**

```bash
nuanced-lsp-morphcloud workspace status /path/to/workspace
```

**Remove a workspace status:**

```bash
nuanced-lsp-morphcloud workspace delete /path/to/workspace
```

This can be useful if you want to recreate the workspace instance.

**List workspaces:**

```bash
nuanced-lsp-morphcloud workspace list
```

## Troubleshooting

**Mutagen sync setup fails:**

Starting a workspace server fails with an error like the following:

```
Connecting to agent (POSIX)...
Error: unable to connect to beta: unable to connect to endpoint: unable to dial agent endpoint: unable to handshake with agent process: unable to receive server magic number: EOF (error output: No user exists for uid 504)
```

Mutagen got in a bad state. Remove `~/.nuanced/mutagen` to have syncs recreated.

## License

This work is licensed under the terms of the MIT license. For a copy, see [LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>.
