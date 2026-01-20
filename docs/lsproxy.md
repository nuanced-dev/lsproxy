# Relation to LSProxy

Nuanced LSP started as a fork of `agentic-labs/lsproxy`. We celebrate and call out the capabilities and contribution from Agentic Labs, and thank them for graciously providing `lsproxy` as an open-source project. We applaud the originality and creativity of using `ast-grep` in combination with LSP capabilities like `find-definition` and `find-references` within a single binary that makes it easy to "proxy" to LSP servers. The Agentic Labs vision of `lsproxy` is still a shining example of what building code intelligence tooling for AI workflows can be, and we are grateful for the opportunity to build on `lsproxy`.

There are some areas in which we found `lsproxy` could be improved to support more flexible operation (i.e. local or cloud), make it easier to tune system resources, make it easier to maintain the project and its associated Dockerfiles, and provide a more efficient runtime.

The following is a summary of the base `lsproxy` implementation and where Nuanced LSP improves on that base.

**Base implementation [agentic-labs/lsproxy](https://github.com/agentic-labs/lsproxy) uses a single process / image model:**

- Single image containing all LSP servers, languages, and system dependencies results in 13.5GB image.
- Distribution requires rebuilding the 13.5GB image.
- Slow image build time for multiple architectures.
- LSP servers and their dependencies are comingled in the same image as the Rust service code. Change one or the other can lead to expensive rebuilds.
- Clients only using one or two LSP servers must still download the full image containing unused LSP servers and dependencies.

**Nuanced LSP and dynamic container orchestration**:

- **Clients only download what they need:** core required images are the proxy (187MB), watchdog (47.7MB), and wrapper (360MB) images, in addition to LSP server images based on workspace composiiton
- **Flexible runtime:** LSP server containers now run as isolated containers, and can run locally or on remote hosts with higher system resource allocation
- **Isolated code changes:** The proxy, watchdog, and wrapper crates are independent from each other, and can be built in parallel. No cascading builds when the Rust code changes.
- **Faster dev loop:** Build only the image needed based on local changes.
- **Docker development loop:** Running Nuanced LSP locally for development is the same runtime and configuration Nuanced LSP uses in production or on end-user hosts. Docker daemon makes it easy to track individual LSP server system resource metrics and indexing latency.
- **LSP server debug loop:** LSP server images are built in isolation, making it easy to test and experiment with LSP servers without cascading image builds.
- **Language version control:** Very detailed language-version support is now possible and included for Ruby / Sorbet. LSP server containers can now be versioned by language versions. This is especially helpful for languages like Ruby.
