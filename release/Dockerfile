FROM --platform=$BUILDPLATFORM rust:1.82.0-slim-bookworm AS builder
ARG BUILDPLATFORM
ARG BUILDARCH
ARG TARGETPLATFORM
ARG TARGETARCH

WORKDIR /usr/src/app

RUN apt-get update && apt install -y --no-install-recommends curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set up cross-compilation tools and target based on build/target platform
RUN apt-get update && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
    if [ "$BUILDARCH" = "arm64" ]; then \
    rustup target add x86_64-unknown-linux-gnu && \
    apt-get install -y gcc-x86-64-linux-gnu && \
    echo '[target.x86_64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
    echo 'linker = "x86_64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
    elif [ "$BUILDARCH" != "amd64" ]; then \
    echo "Unsupported build architecture for linux/amd64: $BUILDARCH" && exit 1; \
    fi \
    ;; \
    "linux/arm64") \
    if [ "$BUILDARCH" = "amd64" ]; then \
    rustup target add aarch64-unknown-linux-gnu && \
    apt-get install -y gcc-aarch64-linux-gnu && \
    echo '[target.aarch64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
    echo 'linker = "aarch64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
    elif [ "$BUILDARCH" != "arm64" ]; then \
    echo "Unsupported build architecture for linux/arm64: $BUILDARCH" && exit 1; \
    fi \
    ;; \
    *) \
    echo "Unsupported target platform: $TARGETPLATFORM (BUILDARCH: $BUILDARCH)" && exit 1 \
    ;; \
    esac && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY . .

# Build with appropriate target
RUN mkdir -p /usr/src/bin

RUN case "$TARGETPLATFORM" in \
    "linux/amd64") \
    if [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release --target x86_64-unknown-linux-gnu && \
    cp target/x86_64-unknown-linux-gnu/release/lsproxy /usr/src/bin/lsproxy; \
    elif [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release && \
    cp target/release/lsproxy /usr/src/bin/lsproxy; \
    fi \
    ;; \
    "linux/arm64") \
    if [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release --target aarch64-unknown-linux-gnu && \
    cp target/aarch64-unknown-linux-gnu/release/lsproxy /usr/src/bin/lsproxy; \
    elif [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release && \
    cp target/release/lsproxy /usr/src/bin/lsproxy; \
    fi \
    ;; \
    esac

RUN curl -O -L "https://go.dev/dl/go1.23.5.linux-${BUILDARCH}.tar.gz" && \
    tar -C /usr/local -xzf go1.23.5.linux-${BUILDARCH}.tar.gz && \
    rm go1.23.5.linux-${BUILDARCH}.tar.gz && \
    GOARCH=${TARGETARCH} /usr/local/go/bin/go install golang.org/x/tools/gopls@latest && \
    if [ "${TARGETARCH}" = "${BUILDARCH}" ]; then \
    cp ~/go/bin/gopls /usr/src/bin/ ; \
    else \
    case "${TARGETARCH}" in \
    "amd64") cp ~/go/bin/linux_amd64/gopls /usr/src/bin/ ;; \
    "arm64") cp ~/go/bin/linux_arm64/gopls /usr/src/bin/ ;; \
    *) echo "Unsupported TARGETARCH: ${TARGETARCH}" && exit 1 ;; \
    esac ; \
    fi

FROM rust:1.81.0-slim-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies, Python, Node.js, PHP, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl3 \
    ca-certificates \
    git \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    clangd \
    build-essential \
    gcc \
    g++ \
    php \
    php-xml \
    php-mbstring \
    php-curl \
    php-zip \
    unzip \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENV HOME=/home/user

# Install Composer
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

# Install PHPActor
RUN cd /usr/src && \
    git clone https://github.com/phpactor/phpactor.git && \
    cd /usr/src/phpactor && \
    composer install

ENV PATH="/usr/src/phpactor/bin:${PATH}"

# Config Python
RUN ln -sf /usr/bin/python3 /usr/bin/python
RUN rm /usr/lib/python3.11/EXTERNALLY-MANAGED

# Install Java
RUN apt-get update && \
    apt-get install -y --no-install-recommends wget gnupg software-properties-common && \
    wget -O - https://packages.adoptium.net/artifactory/api/gpg/key/public | apt-key add - && \
    echo "deb https://packages.adoptium.net/artifactory/deb $(awk -F= '/^VERSION_CODENAME/{print$2}' /etc/os-release) main" | tee /etc/apt/sources.list.d/adoptium.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    temurin-21-jdk \
    gradle \
    maven \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && \
    curl -L -o /tmp/jdt-language-server.tar.gz https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz && \
    mkdir -p /opt/jdtls && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner && \
    rm /tmp/jdt-language-server.tar.gz

# Set JAVA_HOME environment variable
ENV JAVA_HOME=/opt/java/current
ENV PATH=${JAVA_HOME}/bin:${PATH}

# Add jdtls to PATH
ENV PATH="/opt/jdtls/bin:${PATH}"
RUN chmod -R +rw /opt/jdtls/config_*

# Install rust-analyzer
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

# Install go and Gopls
ARG TARGETARCH
RUN curl -O -L "https://go.dev/dl/go1.23.5.linux-${TARGETARCH}.tar.gz" && \
    tar -C /usr/local -xzf go1.23.5.linux-${TARGETARCH}.tar.gz && \
    rm go1.23.5.linux-${TARGETARCH}.tar.gz
COPY --from=builder /usr/src/bin/gopls /usr/bin/gopls

ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# Download and run dotnet install script
RUN curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh -o dotnet-install.sh \
    && chmod +x dotnet-install.sh \
    && ./dotnet-install.sh --channel 8.0 --install-dir /home/user/.dotnet \
    && ./dotnet-install.sh --channel 9.0 --install-dir /home/user/.dotnet \
    && rm dotnet-install.sh

ENV PATH="${PATH}:/home/user/.dotnet"
ENV DOTNET_ROOT=/home/user/.dotnet

# Install csharp-ls globally
RUN dotnet tool install --global csharp-ls

# Add .NET tools to PATH
ENV PATH="${PATH}:/home/user/.dotnet/tools"

# Install Ruby, deps, and LSPs
RUN apt-get update && apt-get install -y \
    zlib1g-dev \
    autoconf \
    bison \
    libyaml-dev \
    libreadline-dev \
    libncurses5-dev \
    libffi-dev \
    libpq-dev \
    libgdbm-dev \
    && rm -rf /var/lib/apt/lists/*

ENV RBENV_ROOT=${HOME}/.rbenv
ENV PATH="$RBENV_ROOT/bin:$RBENV_ROOT/shims:${PATH}"

# Clone rbenv and ruby-build
RUN git clone --depth 1 https://github.com/rbenv/rbenv.git "$RBENV_ROOT" && \
    git clone --depth 1 https://github.com/rbenv/ruby-build.git "$RBENV_ROOT/plugins/ruby-build"

# Install Ruby versions and lsp gems
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" \
    && rbenv install 3.4.2  && rbenv global 3.4.2  && rbenv exec gem install ruby-lsp sorbet  && rbenv rehash \
    && rbenv install 3.4.1 && rbenv shell 3.4.1 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.3.6 && rbenv shell 3.3.6 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.3.5 && rbenv shell 3.3.5 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.2.6 && rbenv shell 3.2.6 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.2.2 && rbenv shell 3.2.2 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash

# Copy binary with -bin suffix
COPY --from=builder /usr/src/bin/lsproxy /usr/local/bin/lsproxy-bin

# Copy ast-grep rules
COPY ./src/ast_grep /usr/src/ast_grep

EXPOSE 4444
ENV RA_LOG="/tmp/rust-analyzer.log"

RUN mkdir -p /mnt/workspace /usr/local/cargo /usr/src/app /usr/local/go /home/user/go \
    && chown -R 1000:1000 /mnt/workspace /usr/local/cargo /usr/src/app /usr/local/go /home/user /usr/local/bin

# Create the wrapper script with all current env vars
RUN echo '#!/bin/bash' > /usr/local/bin/lsproxy \
    && env | while read -r line; do \
    echo "export $line" >> /usr/local/bin/lsproxy; \
    done \
    && echo 'eval "$(rbenv init -)"' >> /usr/local/bin/lsproxy \
    && echo 'exec /usr/local/bin/lsproxy-bin "$@"' >> /usr/local/bin/lsproxy \
    && chmod +x /usr/local/bin/lsproxy

CMD ["lsproxy"]
