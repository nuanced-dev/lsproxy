# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2025-01-20

### Added

- A new `server` command runs Nuanced LSP as an LSP server.

### Changed

- Updated service dependency to version to 0.5.0.

- The `pull` method now follows the command, and supports pulling service
  and/or language images without having to construct the iamge names.

## [0.4.0] - 2025-01-14

### Changed

- The `up` command's `languageContainerVersion` argument
  and `--language-container-version` flag are renamed to
  `languageImageVersion` and `--language-image-version`, respectively.

- The `up` command's `{proxy,watchdog,wrapper}Image` argument and
  `--{proxy,watchdog,wrapper}-image` flag have been removed in favor
  of `serviceImageVersion` and `containerRegistry` arguments, and
  `--service-image-version` and `--container-registry` flags.

- The `pull` command now makes it easier to pull service and language
  images without the user having to know the full image names. It accepts
  the same version and registry flags as the `up` command.

- Updates the used service image version to 0.4.9.

- The `DEFAULT_{PROXY,WATCHDOG,WRAPPER}_IMAGE` constants have been
  removed in favor of `DEFAULT_{SERVICE,LANGUAGE}_IMAGE_VERSION` and
  `DEFAULT_CONTAINER_REGISTRY`.

- The client always tries to use local images before falling back to
  pulling an using images from the registry.

## [0.3.18] - 2025-11-25

Nuanced LSP rearchitecture release for dynamic container orchestration.

- `nuanced-lsp` version 0.3.18
- `nuanced-lsp-proxy` image version 0.4.8
- `nuanced-lsp-watchdog` image version 0.4.8
- `nuanced-lsp-wrapper` image version 0.4.8
- Version 1.0.0 for Nuanced language images

## [0.3.16] - 2025-10-16

### Changed

lsproxy-version bumped from 0.3.15 to 0.4.6. This now completes adoption of separate lsproxy versioning.

## [0.3.15] - 2025-09-29

### Changed

- Changes package name from @nuanced-dev/nuanced-lsp-ts to @nuanced-dev/lsp.
- Updated binary from nuanced-lsp-ts to nuanced-lsp.
- Track LSProxy image version independently from nuanced-lsp version.

## [0.3.14] - 2025-09-23

### Changed

- Builds and exports CJS alongside existing ESM modules.
