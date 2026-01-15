# Java LSP server container (jdtls - Eclipse JDT Language Server)
# Java 21 (LTS) - For other versions, see versioned Dockerfiles (java-8, java-11, java-17, etc.)
# Multi-stage build to minimize image size

# Builder stage: Download jdtls
FROM debian:bookworm-slim AS builder
LABEL org.opencontainers.image.source https://github.com/nuanced-dev/lsp

ENV DEBIAN_FRONTEND=noninteractive

# Install curl for downloading jdtls
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Download and extract jdtls
RUN curl -L -o /tmp/jdt-language-server.tar.gz https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz && \
    mkdir -p /opt/jdtls && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner && \
    rm /tmp/jdt-language-server.tar.gz

# Runtime stage: Pure Debian base (standalone image with language-specific LSP server)
# Wrapper binary will be mounted at runtime via --volumes-from
# Java is JVM-based, doesn't need build tools
FROM debian:bookworm-slim
LABEL org.opencontainers.image.source https://github.com/nuanced-dev/lsp

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install minimal runtime dependencies
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    wget \
    gnupg \
    software-properties-common \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Temurin JDK 21 from Adoptium
RUN wget -O - https://packages.adoptium.net/artifactory/api/gpg/key/public | gpg --dearmor -o /usr/share/keyrings/adoptium-archive-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/adoptium-archive-keyring.gpg] https://packages.adoptium.net/artifactory/deb $(awk -F= '/^VERSION_CODENAME/{print$2}' /etc/os-release) main" | tee /etc/apt/sources.list.d/adoptium.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends temurin-21-jdk && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy jdtls from builder
COPY --from=builder /opt/jdtls /opt/jdtls

# Set JAVA_HOME dynamically based on the actual installed JDK directory
# The JDK installs to /usr/lib/jvm/temurin-21-jdk-{arch} where arch is arm64 or amd64
# We find the actual directory and create a symlink for consistent PATH handling
RUN JAVA_ARCH=$(dpkg --print-architecture) && \
    ln -s /usr/lib/jvm/temurin-21-jdk-${JAVA_ARCH} /usr/lib/jvm/temurin-21-jdk

ENV JAVA_HOME=/usr/lib/jvm/temurin-21-jdk
ENV PATH=${JAVA_HOME}/bin:/opt/jdtls/bin:${PATH}

# Set permissions on jdtls config directories
RUN chmod -R +rw /opt/jdtls/config_*

# Create symlink for ast-grep in standard PATH location
RUN ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="java"

# Copy and setup Java-specific entrypoint script
COPY dockerfiles/entrypoints/java-entrypoint.sh /usr/local/bin/java-entrypoint.sh
RUN chmod +x /usr/local/bin/java-entrypoint.sh

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# Use custom entrypoint that finds launcher jar and sets up jdtls workspace
# NOTE: Java uses a custom entrypoint instead of wrapper ENTRYPOINT
ENTRYPOINT ["/usr/local/bin/java-entrypoint.sh"]
CMD []
