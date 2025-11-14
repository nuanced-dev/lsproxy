#!/usr/bin/env bash

set -e  # Exit immediately if a command exits with a non-zero status

./scripts/build.sh

# Run the application to generate the OpenAPI spec
docker run --name temp_lsp_box -v "$(pwd)/lsproxy/target/release":/usr/src/app lsproxy-dev ./lsproxy -w

# Copy the generated OpenAPI spec from the container to the host
docker cp temp_lsp_box:/usr/src/app/openapi.json ./openapi.json

# Remove the temporary container
docker rm temp_lsp_box

echo "OpenAPI specification has been generated and saved as openapi.json"
