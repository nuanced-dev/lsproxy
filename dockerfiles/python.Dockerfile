# Python LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Python and jedi-language-server
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install Python and pip (Debian Bookworm includes Python 3.11)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create virtual environment and install jedi-language-server
RUN python3 -m venv /opt/jedi-venv && \
    /opt/jedi-venv/bin/pip install --no-cache-dir \
    jedi-language-server

# Runtime stage: Use build-enabled base for potential native extensions
FROM lsproxy-base-build:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install Python runtime (without pip/venv to save space)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    python3 \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create python symlink for compatibility
RUN ln -sf /usr/bin/python3 /usr/bin/python

# Copy virtual environment from builder
COPY --from=builder /opt/jedi-venv /opt/jedi-venv

# Add jedi-language-server to PATH
ENV PATH="/opt/jedi-venv/bin:${PATH}"

# Set workspace path (must match mount point)
WORKDIR /mnt/workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "jedi-language-server"]
