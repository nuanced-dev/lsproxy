#!/bin/bash
set -e

# Create jdtls workspace directory with proper permissions
mkdir -p /tmp/jdtls_workspace
chmod 777 /tmp/jdtls_workspace

# Find the launcher jar dynamically
LAUNCHER_JAR=$(find /opt/jdtls/plugins -name 'org.eclipse.equinox.launcher_*.jar' | head -n 1)

if [ -z "$LAUNCHER_JAR" ]; then
    echo "ERROR: Could not find launcher jar in /opt/jdtls/plugins"
    exit 1
fi

echo "Using launcher jar: $LAUNCHER_JAR"

# Build the command with all required arguments
exec /usr/local/bin/lsp-wrapper \
    --lsp-command java \
    --lsp-arg=-Declipse.application=org.eclipse.jdt.ls.core.id1 \
    --lsp-arg=-Dosgi.bundles.defaultStartLevel=4 \
    --lsp-arg=-Declipse.product=org.eclipse.jdt.ls.core.product \
    --lsp-arg=-Dlog.protocol=true \
    --lsp-arg=-Dlog.level=ALL \
    --lsp-arg=-Xmx1g \
    --lsp-arg=--add-modules=ALL-SYSTEM \
    --lsp-arg=--add-opens \
    --lsp-arg=java.base/java.util=ALL-UNNAMED \
    --lsp-arg=--add-opens \
    --lsp-arg=java.base/java.lang=ALL-UNNAMED \
    --lsp-arg=-jar \
    --lsp-arg="$LAUNCHER_JAR" \
    --lsp-arg=-configuration \
    --lsp-arg=/opt/jdtls/config_linux \
    --lsp-arg=-data \
    --lsp-arg=/tmp/jdtls_workspace
