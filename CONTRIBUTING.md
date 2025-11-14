# Contributing Guide

We welcome all kinds of contributions. _You don't need to be an expert
in API or rust development to help out._

Contributions to the build-test-run system and github workflows are also welcome! We love to learn from the expertise of the community 😄

## Checklist

Contributions are made through
[pull requests](https://help.github.com/articles/using-pull-requests/) from a fork of the repo.
Before sending a pull request, make sure to do the following:

- [Build and format](#build-format) your code
- [Write tests](#tests)
- [Run tests](#tests) and check that they pass

_Please reach out to the lsproxy team before starting work on a large
contribution._ Get in touch at
[GitHub issues](https://github.com/agentic-labs/lsproxy/issues)
or [on Discord](https://discord.gg/WafeS3jN).

## Prerequisites

To build lsproxy from source, all you need is docker. But rust + rust-analyzer is recommended for getting lints in your IDE:

- [Docker installation instructions](https://docs.docker.com/engine/install/)
- [Rust installation instructions](https://www.rust-lang.org/tools/install)

## <a name="build-format">Building and formatting</a>

Building using docker is pretty simple. We have a build script for this purpose.

```
./scripts/build.sh
```

> :warning: The build script executes the build inside the docker container. It is not recommended and we do not officially support building `lsproxy` locally.

Before committing, make sure to format the code with

```
./scripts/fmt.sh
```

## Running locally

First, build the service container:

```bash
./scripts/build-rust-containers.sh
```

Then start the service with your workspace:

```bash
./scripts/start-service.sh $WORKSPACE_PATH
```

> :warning: The service runs `lsproxy` inside Docker containers, we do not support running `lsproxy` directly on the host.

## <a name="tests">Testing</a>

Good testing is essential to ensuring `lsproxy` continues to work for everyone! Whenever you're adding features or changing code please make sure to:
- Create new tests for any features or cases you add
- Ensure existing tests all pass

Test writing and adjustments should be done **before** opening a PR.

---

To run tests:

```
./scripts/test.sh
```

Ideally we would like to keep code coverage at the same level or higher. You can generate a code coverage report with the following:
```
./scripts/coverage_test.sh
```

This will create a coverage report that can be viewed with

```
open lsproxy/target/llvm-cov/html/index.html
```

Thanks in advance for your contributions and, most importantly, HAVE FUN!
