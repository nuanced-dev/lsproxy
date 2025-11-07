<div align="center">

# Nuanced LSProxy - Precise code navigation via an API

[![License](https://img.shields.io/github/license/nuanced-dev/lsproxy)](LICENSE)

**Forked from [agentic-labs/lsproxy](https://github.com/agentic-labs/lsproxy)**

</div>



## <a name="what-is-lsproxy">What is lsproxy?</a>

`lsproxy` offers IDE-like code analysis and navigation functionality in a docker container with a REST API.

It supports [multiple languages](#supported-languages) and resolves relationships between code symbols (functions, classes, variables) anywhere in the project.

`lsproxy` runs [Language Servers](https://microsoft.github.io/language-server-protocol/) and [ast-grep](https://github.com/ast-grep/ast-grep) under the hood, giving you precise search results without the headache of configuring and integrating language-specific tooling.

For more info, please refer to our [API Reference](https://docs.nuanced.dev/lsp/overview).

[![](https://mermaid.ink/img/pako:eNptUtFumzAU_RV0q0qdRKpAgAAPk6buZVInTau0h9ZV5YRrYhVsZJuuLMq_7xraNLQ1D9jnnHt8ru09bHWFUIJo9N_tjhsXXP9miqmAhqfuLhhc0X_DLTL4cu-5ibX9pja825FMOS4VmjsGje2Mfh4Y3E8iP55007ej0Z9xNtm8slRBdddc1T2vMbhB84TGzgx4TQpu3aI22M2ZThL17dePTzYMFouv3v1TnNezBBPWydM95xiq6tj40D5I9SBkg75jaZ2HNrqxgVSBh49pKhSWNEKq6kXjIamkk1odVeajiiA0qLY4ncTpjVCugMFP3SvHYAw59TUpKPCInYScEz7SHDEjMmHn54F1Q4Nvl-obasozzMRKiNA6ox-xPEt4scT4Xc1O01FMcpH6771nI1E5-yYRIoUQWjQtlxU9wr0vYOB26F9JSdOKm0cGTB1Ix3unbwa1hdKZHkPou4o7_C45PcMWSsEbS2jH1a3W7auIllDu4RnKJLtM0yJL82hdJKs4zUIYoIyj5WWeJlGyzKNslefr5BDCv9GAiCKOi6yIlnGeFkmxPvwHnPP5bQ?type=png)](https://mermaid.live/edit#pako:eNptUtFumzAU_RV0q0qdRKpAgAAPk6buZVInTau0h9ZV5YRrYhVsZJuuLMq_7xraNLQ1D9jnnHt8ru09bHWFUIJo9N_tjhsXXP9miqmAhqfuLhhc0X_DLTL4cu-5ibX9pja825FMOS4VmjsGje2Mfh4Y3E8iP55007ej0Z9xNtm8slRBdddc1T2vMbhB84TGzgx4TQpu3aI22M2ZThL17dePTzYMFouv3v1TnNezBBPWydM95xiq6tj40D5I9SBkg75jaZ2HNrqxgVSBh49pKhSWNEKq6kXjIamkk1odVeajiiA0qLY4ncTpjVCugMFP3SvHYAw59TUpKPCInYScEz7SHDEjMmHn54F1Q4Nvl-obasozzMRKiNA6ox-xPEt4scT4Xc1O01FMcpH6771nI1E5-yYRIoUQWjQtlxU9wr0vYOB26F9JSdOKm0cGTB1Ix3unbwa1hdKZHkPou4o7_C45PcMWSsEbS2jH1a3W7auIllDu4RnKJLtM0yJL82hdJKs4zUIYoIyj5WWeJlGyzKNslefr5BDCv9GAiCKOi6yIlnGeFkmxPvwHnPP5bQ)

## Key Features

- 🎯 **Precise Cross-File Code Navigation**: Find symbol definitions and references across your entire project.
- 🌐 **Unified API**: Access multiple language servers through a single API.
- 🛠️ **Auto-Configuration**: Automatically detect and configure language servers based on your project files.
- 📊 **Code Diagnostics**: (Coming Soon) Get language-specific lint output from an endpoint.
- 🌳 **Call & Type Hierarchies**: (Coming Soon) Query multi-hop code relationships computed by the language servers.
- 🔄 **Procedural Refactoring**: (Coming Soon) Perform symbol operations like `rename`, `extract`, `auto import` through the API.
- 🧩 **SDKs**: Libraries to get started calling `lsproxy` in popular languages.


## <a name="getting-started">Getting started</a>
The easiest way to get started is to run our tutorial! Check it out at [demo.lsproxy.dev](https://demo.lsproxy.dev)
It's also super easy to run `lsproxy` on your code! We keep the latest version up to date on Docker Hub, and we have a Python SDK available via `pip.`

### Quick Start

```bash
# 1. Build all containers (one-time setup)
./scripts/build-all-containers.sh

# 2. Start the service
./scripts/start-service.sh

# 3. Run tests
cargo test --workspace           # Unit tests
./scripts/test-all-endpoints.sh  # Integration tests
```

See [QUICKSTART.md](QUICKSTART.md) for detailed instructions.

### Architecture

LSProxy uses a **service container** that dynamically spawns **language-specific containers**. This provides massive space savings compared to the original monolithic implementation.

#### Container Architecture Example

When you load a workspace with Python and TypeScript files, here's what happens:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Client Application                          │
│                  (API calls to localhost:4444)                  │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
         ┌───────────────────────────────────────┐
         │  lsproxy-service (Orchestrator)       │
         │  • Size: 187MB                        │
         │  • Built from: crates/orchestrator    │
         │  • Routes requests to language        │
         │    containers                         │
         │  • Manages container lifecycle        │
         └──────┬──────────────────┬─────────────┘
                │                  │
       ┌────────▼────────┐    ┌────▼──────────────┐
       │ lsproxy-python  │    │ lsproxy-typescript│
       │ • Size: 791MB   │    │ • Size: 1.0GB     │
       │ • jedi-ls       │    │ • typescript-ls   │
       │ • ast-grep      │    │ • ast-grep        │
       │ • lsp-wrapper   │    │ • lsp-wrapper     │
       └─────────────────┘    └───────────────────┘

       ┌───────────────────────────────────────────────────────────┐
       │ lsproxy-watchdog (Independent Monitor)                    │
       │ • Size: 47.3MB                                            │
       │ • Monitors service container status via docker inspect    │
       │ • On service crash: Cleans up all language containers     │
       │ • Uses Docker labels to find orphaned containers          │
       └───────────────────────────────────────────────────────────┘

       Total image size on disk required for this workspace: 2.0GB (service + python + typescript + watchdog)
```

#### Container Components

**1. Service Container (lsproxy-service)** - 187MB
- Built from: `dockerfiles/service.Dockerfile` → `crates/orchestrator`
- Thin HTTP handlers that proxy requests to language containers
- Detects languages in workspace and spawns only needed containers
- Manages container lifecycle and routing

**2. Language Containers** - Variable sizes (see table below)
- Built from: Language-specific Dockerfiles → `crates/wrapper`
- Each contains: Language server + ast-grep + lsp-wrapper
- Full LSP communication handlers with stdio→HTTP translation
- Only spawned when language files are detected in workspace

**3. Watchdog Container (lsproxy-watchdog)** - 47.3MB
- Built from: `dockerfiles/watchdog.Dockerfile`
- Monitors service container health
- Automatically cleans up language containers on service crash
- Minimal footprint for reliability

**4. Common Library (lsproxy-common)**
- Not a container - compiled into orchestrator and wrapper
- Shared types, utilities, AST-grep integration
- Zero runtime overhead, zero duplication

### Development Workflow

```bash
# Start service with your workspace
./scripts/start-service.sh /path/to/your/project --logs

# In another terminal, make changes and test
curl http://localhost:4444/v1/system/health | jq

# Run comprehensive tests
./scripts/test-all-endpoints.sh

# Check what's running
docker ps --filter "name=lsproxy-"

# View logs
docker logs -f lsproxy-service

# Stop when done
docker rm -f lsproxy-service
```

### Available Scripts

| Script | Purpose |
|--------|---------|
| `scripts/start-service.sh` | Start LSProxy service with workspace |
| `scripts/build-all-containers.sh` | Build all language containers |
| `scripts/test-all-endpoints.sh` | Test all endpoints for all languages |
| `scripts/test-container-lifecycle.sh` | Test container orchestration |

### Language Container Sizes

Each language container is built on top of the base image (739MB) which includes lsp-wrapper and ast-grep:

| Language | Container | Dockerfile | Image Size | Language Server |
|----------|-----------|------------|------------|----------------|
| Python | `lsproxy-python` | `dockerfiles/python.Dockerfile` | 791MB | jedi-language-server |
| TypeScript/JavaScript | `lsproxy-typescript` | `dockerfiles/typescript.Dockerfile` | 1.0GB | typescript-language-server |
| Golang | `lsproxy-golang` | `dockerfiles/golang.Dockerfile` | 1.15GB | gopls |
| C/C++ | `lsproxy-clangd` | `dockerfiles/clangd.Dockerfile` | 1.11GB | clangd |
| PHP | `lsproxy-php` | `dockerfiles/php.Dockerfile` | 944MB | phpactor |
| Ruby | `lsproxy-ruby-3.4.4` | `dockerfiles/ruby-3.4.4.Dockerfile` | 1.14GB | solargraph |
| Ruby (Sorbet) | `lsproxy-ruby-sorbet-3.4.4` | `dockerfiles/ruby-sorbet-3.4.4.Dockerfile` | 1.17GB | sorbet |
| Rust | `lsproxy-rust` | `dockerfiles/rust.Dockerfile` | 1.57GB | rust-analyzer |
| Java | `lsproxy-java` | `dockerfiles/java.Dockerfile` | 1.57GB | eclipse-jdtls |
| C# | `lsproxy-csharp` | `dockerfiles/csharp.Dockerfile` | 2.66GB | omnisharp |

**Base Image**: `lsproxy-base` (739MB) - Contains lsp-wrapper binary and ast-grep, inherited by all language containers

### Why This Architecture Saves Space

**Monolithic Approach (Original)**: 13.3GB single image
- Contains ALL language servers and dependencies
- Must download and store 13.3GB even for a single-language project
- Updates require rebuilding entire 13.3GB image

**Container Orchestration (This Fork)**: ~2-4GB typical usage
- Service container (187MB) + only needed language containers
- **Example 1**: Python-only project = 187MB + 791MB + 47MB = **1.0GB** (92% savings)
- **Example 2**: Python + TypeScript project = 187MB + 791MB + 1.0GB + 47MB = **2.0GB** (85% savings)
- **Example 3**: All 10 languages = 187MB + 11.6GB + 47MB = **11.8GB** (11% savings)
- Updates only rebuild changed language containers (~1GB each)

**Additional Benefits**:
- Parallel container builds (faster CI/CD)
- Language containers can be cached independently
- Easier to add new language support without affecting others
- Better resource isolation and crash recovery via watchdog

### Documentation

- [QUICKSTART.md](QUICKSTART.md) - Get running in 3 minutes
- [TESTING.md](TESTING.md) - Comprehensive testing guide

## <a name="supported-languages">Supported languages</a>

We're looking to add new language support or better language servers so let us know what you need!
|Language|Server|URL|
|:-|:-|:-|
|C/C++|`clangd`|https://clangd.llvm.org/|
|C#|`omnisharp`|https://github.com/OmniSharp/csharp-language-server-protocol|
|Golang|`gopls`|https://github.com/golang/tools/tree/master/gopls|
|Java|`jdtls`|https://github.com/eclipse-jdtls/eclipse.jdt.ls|
|Javascript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
|PHP|`phpactor`|https://github.com/phpactor/phpactor|
|Python|`jedi-language-server`|https://github.com/pappasam/jedi-language-server|
|Ruby|`sorbet`|https://sorbet.org/docs/lsp|
|Rust|`rust-analyzer`|https://github.com/rust-lang/rust-analyzer|
|Typescript|`typescript-language-server`|https://github.com/typescript-language-server/typescript-language-server|
|Your Favorite Language | Awesome Language Server | https://github.com/nuanced-dev/lsproxy/issues/new |
