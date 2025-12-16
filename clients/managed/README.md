# Morph Cloud Examples

A collection of TypeScript examples demonstrating [Morph Cloud](https://cloud.morph.so)'s snapshot and instance management capabilities.

## Overview

This project contains three example scripts that showcase Morph Cloud's ability to capture and restore complete computational states:

1. **Counter Example** (`index.ts`) - Demonstrates how background processes persist across snapshots
2. **Clone & Build** (`clone-and-build.ts`) - Shows how build artifacts are preserved in snapshots
3. **Cleanup** (`cleanup.ts`) - Utility to delete all instances and snapshots

## Prerequisites

Before running these examples, you'll need:

1. **Morph Cloud API Key**
   - Sign up at [cloud.morph.so](https://cloud.morph.so)
   - Generate an API key from your dashboard

2. **GitHub Personal Access Token** (for clone-and-build example only)
   - Create a token at [github.com/settings/tokens](https://github.com/settings/tokens)
   - Requires `repo` scope for private repositories

## Installation

```bash
# Clone this repository
git clone https://github.com/nuanced-dev/morph-example.git
cd morph-example

# Install dependencies
npm install

# Build the TypeScript project
npm run build
```

## Configuration

Set your environment variables:

```bash
# Required for all examples
export MORPH_API_KEY='your_morph_api_key_here'

# Required only for clone-and-build example
export GITHUB_TOKEN='your_github_token_here'
```

## Examples

### 1. Counter Example

**What it does:**
- Creates a snapshot and starts an instance
- Starts a background counter that increments every 5 seconds
- Creates a snapshot of the running instance (with the counter running)
- Starts a new instance from that snapshot
- Verifies the counter continues from where it left off

**What it demonstrates:**
- Background processes survive snapshots
- Process state is perfectly preserved
- New instances resume execution seamlessly

**How to run:**
```bash
npm run counter
```

**Expected output:**
```
Created snapshot: snapshot_xxxxx
Started instance: morphvm_xxxxx
Initial counter: 1
Counter after waiting: 3
Created snapshot: snapshot_yyyyy
Started new instance: morphvm_yyyyy
Counter in new instance: 5
✨ Success! The background process continued exactly where it left off.
```

---

### 2. Clone & Build Example

**What it does:**
1. Creates a base snapshot with a minimal VM image
2. Installs development tools (Git, GitHub CLI, Bun, Python)
3. Clones a private GitHub repository
4. Runs the full build process (install dependencies + compile)
5. Creates a snapshot of the built project
6. Starts a new instance from the snapshot
7. Reruns the build to verify everything is cached

**What it demonstrates:**
- Complete development environments can be snapshotted
- Build artifacts and dependencies are preserved
- Subsequent builds are nearly instant (no reinstall needed)
- Starting from a snapshot is much faster than rebuilding

**Configuration:**

Edit `src/clone-and-build.ts` to customize:

```typescript
const CONFIG = {
  githubToken: process.env.GITHUB_TOKEN,
  repo: 'your-org/your-repo',  // Change this to your repo
  clonePath: '/root/project',
  buildCommands: [
    'bun install',    // Change to npm/yarn if needed
    'bun run build'
  ],
  verifyCommand: 'bun run build --ignore-scripts',
};
```

**How to run:**
```bash
export GITHUB_TOKEN='your_token'
export MORPH_API_KEY='your_key'
npm run clone-and-build
```

**Expected output:**
```
📦 Step 1: Creating base snapshot...
✅ Created base snapshot: snapshot_xxxxx

🔧 Step 2: Starting instance from base snapshot...
✅ Started instance: morphvm_xxxxx

🔨 Step 3: Installing git, GitHub CLI, and build tools...
✅ Installed Python 3.x
✅ Installed gh version 2.83.0
✅ Installed Bun 1.3.1

📥 Step 4: Cloning private repository...
✅ Successfully cloned repository

🔨 Step 5: Building the repository...
✅ First build completed in 69.08s

📸 Step 6: Creating snapshot of built repository...
✅ Created built snapshot: snapshot_yyyyy

🔄 Step 7: Starting new instance from built snapshot...
✅ Started new instance: morphvm_zzzzz

🔍 Step 8: Verifying build state is preserved...
✅ Verification build completed in 1.23s

📊 Results Summary:
═══════════════════════════════════════════════════════
   First build time:        69.08s
   Verification build time: 1.23s
   Time saved:              67.85s
   Speedup:                 56.1x faster
═══════════════════════════════════════════════════════
```

---

### 3. Cleanup Utility

**What it does:**
- Lists all running instances and stops them
- Lists all snapshots and deletes them
- Provides progress feedback with error handling

**When to use:**
- After testing/experimenting with Morph Cloud
- To clean up resources and avoid charges
- When you want to start fresh

**How to run:**
```bash
npm run cleanup
```

**Expected output:**
```
🧹 Starting cleanup of all instances and snapshots...

📋 Fetching all instances...
Found 3 instance(s):

   🗑️  Deleting instance morphvm_xxxxx (status: ready)...
   ✅ Deleted instance morphvm_xxxxx
   🗑️  Deleting instance morphvm_yyyyy (status: ready)...
   ✅ Deleted instance morphvm_yyyyy
   🗑️  Deleting instance morphvm_zzzzz (status: paused)...
   ✅ Deleted instance morphvm_zzzzz

📋 Fetching all snapshots...
Found 5 snapshot(s):

   🗑️  Deleting snapshot snapshot_xxxxx (status: ready)...
   ✅ Deleted snapshot snapshot_xxxxx
   ...

✨ Cleanup complete!
```

## Available Scripts

```bash
# Build the TypeScript project
npm run build

# Run the counter example
npm run counter

# Run the clone-and-build example
npm run clone-and-build

# Clean up all instances and snapshots
npm run cleanup
```

## Project Structure

```
morph-example/
├── src/
│   ├── counter.ts         # Counter example
│   ├── clone-and-build.ts # Clone & build example
│   └── cleanup.ts         # Cleanup utility
├── dist/                  # Compiled JavaScript (generated)
├── package.json
├── tsconfig.json
└── README.md
```

## Troubleshooting

### "API key not found" error
Make sure you've set the `MORPH_API_KEY` environment variable:
```bash
export MORPH_API_KEY='your_key_here'
```

### "Failed to clone repository" error
1. Verify your `GITHUB_TOKEN` is set
2. Ensure the token has `repo` scope
3. Check that the repo URL in `CONFIG` is correct

### Build errors with native modules
The clone-and-build script installs Python and build tools automatically. If you still encounter issues:
- Check that `python3` and `build-essential` are installed
- Verify the `PYTHON` environment variable is set to `/usr/bin/python3`

### Slow verification build
If the verification build is still slow (>5s), prebuild scripts may be regenerating files. Make sure `verifyCommand` uses `--ignore-scripts`:
```typescript
verifyCommand: 'bun run build --ignore-scripts'
```

## Learn More

- [Morph Cloud Documentation](https://cloud.morph.so/docs)
- [Morph Cloud API Reference](https://cloud.morph.so/docs/api-reference)
- [morphcloud npm package](https://www.npmjs.com/package/morphcloud)

## License

ISC
