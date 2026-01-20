import { VERSION } from "./__generated/version.js";

// VERSION is the package version
export { VERSION };

// Container registry for published images
export const DEFAULT_CONTAINER_REGISTRY = "ghcr.io/nuanced-dev";

// Language image version
export const DEFAULT_LANGUAGE_IMAGE_VERSION = "1";

// Service image version
export const DEFAULT_SERVICE_IMAGE_VERSION = "0.5.0";

// DEFAULT_BIND_HOST is the default host address to which the LSProxy container binds.
// The default is the local loopback, implying the container only accepts connections from the same machine.
// See DEFAULT_HOST_URL for the client-side counterpart.
export const DEFAULT_BIND_HOST = "127.0.0.1";

// DEFAULT_CONTAINER_NAME is the default name for the LSProxy container.
export const DEFAULT_CONTAINER_NAME = "nuanced-lsp";

// DEFAULT_CONTAINER_PORT is the default port where the LSProxy container listens for incoming requests. The default LSProxy container port is 4444.
export const DEFAULT_CONTAINER_PORT = 4444;

// DEFAULT_HOST_PORT is the default port where the LSProxy container is expected to be reachable from the client.
export const DEFAULT_HOST_PORT = 4444;

// DEFAULT_HOST_URL is the default host URL where the LSProxy container is expected to be reachable from the client.
// The default is the local loopback, implying the container is running on the same machine as the client.
// See DEFAULT_BIND_HOST for the container-side counterpart.
export const DEFAULT_HOST_URL = "http://127.0.0.1";

// DEFAULT_MOUNT_DIR is the default directory inside the container where the host workspace is mounted.
export const DEFAULT_MOUNT_DIR = "/mnt/workspace";

// DEFAULT_RETRIES is the number of retries a client will attempt HTTP requests to the LSProxy container before returning a result or error.
export const DEFAULT_RETRIES = 5;

// DEFAULT_TIMEOUT_SECS is the per-request timeout in seconds for HTTP requests from the client to the LSProxy container.
export const DEFAULT_TIMEOUT_SECS = 120;

// Image base names (without registry prefix or version tag)
export const PROXY_IMAGE_BASE = "nuanced-lsp-proxy";
