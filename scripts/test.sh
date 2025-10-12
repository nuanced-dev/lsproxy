#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Build the application using the build Dockerfile
docker build -t lsproxy-dev lsproxy

# Run cargo tests with Docker-in-Docker support
# Mount Docker socket to allow tests to spawn language containers
# Mount workspace directory for test files
# Add host.docker.internal mapping for container-to-container communication
if ! docker run --rm \
    -v "$(pwd)/lsproxy":/usr/src/app \
    -v "$(pwd)":/mnt/lsproxy_root \
    -v /var/run/docker.sock:/var/run/docker.sock \
    --add-host=host.docker.internal:host-gateway \
    -e HOST_WORKSPACE_PATH=/mnt/lsproxy_root/sample_project \
    lsproxy-dev cargo test --target-dir /tmp/target $@; then
    echo "Tests failed. Exiting."
    exit 1
fi
