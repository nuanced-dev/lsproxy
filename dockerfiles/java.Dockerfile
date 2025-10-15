# Java LSP server container (jdtls - Eclipse JDT Language Server)
# Java 21 (LTS) - For other versions, see versioned Dockerfiles (java-8, java-11, java-17, etc.)
# Multi-stage build to minimize image size

# Builder stage: Download jdtls
FROM debian:bookworm-slim AS builder

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

# Runtime stage: Use slim base (Java is JVM-based, doesn't need build tools)
FROM lsproxy-base:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install Temurin JDK 21 from Adoptium
RUN apt-get update && \
    apt-get install -y --no-install-recommends wget gnupg software-properties-common && \
    wget -O - https://packages.adoptium.net/artifactory/api/gpg/key/public | gpg --dearmor -o /usr/share/keyrings/adoptium-archive-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/adoptium-archive-keyring.gpg] https://packages.adoptium.net/artifactory/deb $(awk -F= '/^VERSION_CODENAME/{print$2}' /etc/os-release) main" | tee /etc/apt/sources.list.d/adoptium.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends temurin-21-jdk && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy jdtls from builder
COPY --from=builder /opt/jdtls /opt/jdtls

# Set JAVA_HOME and PATH
ENV JAVA_HOME=/usr/lib/jvm/temurin-21-jdk-arm64
ENV PATH=${JAVA_HOME}/bin:/opt/jdtls/bin:${PATH}

# Set permissions on jdtls config directories
RUN chmod -R +rw /opt/jdtls/config_*

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="java"

# Copy and setup Java-specific entrypoint script
COPY dockerfiles/entrypoints/java-entrypoint.sh /usr/local/bin/java-entrypoint.sh
RUN chmod +x /usr/local/bin/java-entrypoint.sh

# Set workspace path
WORKDIR /mnt/workspace

# Use custom entrypoint that finds launcher jar and sets up jdtls workspace
ENTRYPOINT ["/usr/local/bin/java-entrypoint.sh"]
CMD []
