# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.9] - 2025-01-15

### Changes

- The proxy will use service and language images with a specific
  version, instead of defaulting to `latest`, ensuring compatible
  service containers. The versions are (a) the crate and language
  versions in the repo, or (b) the values of `SERVICE_IMAGE_VERSION`
  and `LANGUAGE_IMAGE_VERSION` environment variables at build time, or
  (c) the values of those variables at run time.

- The container registry where the proxy looks for images can be overriden
  with the `CONTAINER_REGISTRY` environment variable at build time or
  run time. The default is GHCR.

- Proxy initialization changed to allow failing fast if other containers fail to start. The proxy shuts down immediately if other service containers fail to start. The language status reported by the health endpoint now reports healthy / unhealthy. Previously a false status could also mean the language was still initializing.

- The proxy will now always try to use local images when starting containers, before falling back to pulling and using images from the registry.
