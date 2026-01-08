# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Renames the `languageContainerVersion` API argument and `--language-container-version` CLI flag to the `up` command to `languageImageVersion` and `--language-image-version`, respectively.
- Updates `nuanced-lsp-proxy`, `nuanced-lsp-watchdog`, and `nuanced-lsp-wrapper` image versions to 0.4.8.

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
