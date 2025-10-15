FROM rust:1.82.0-slim-bookworm

# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime and development dependencies, Python, Node.js, PHP, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libssl3 \
    ca-certificates \
    git \
    procps \
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

# Install Composer
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

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

# Add jdtls to PATH
ENV PATH="/opt/jdtls/bin:${PATH}"
RUN chmod -R +rw /opt/jdtls/config_*

# Install rust-analyzer and rustfmt
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install coverage tools
RUN cargo install cargo-llvm-cov
RUN rustup component add llvm-tools-preview

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

# Install go and Gopls
ARG TARGETARCH
RUN curl -O -L "https://go.dev/dl/go1.24.2.linux-${TARGETARCH}.tar.gz" && \
    tar -C /usr/local -xzf go1.24.2.linux-${TARGETARCH}.tar.gz && \
    rm go1.24.2.linux-${TARGETARCH}.tar.gz && \
    /usr/local/go/bin/go install golang.org/x/tools/gopls@latest && \
    cp ~/go/bin/gopls /usr/bin/gopls

ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# -----------------------------
# Ruby via rbenv (multi-version)
# -----------------------------
# Build deps for Ruby
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

# Ensure home dir exists (we'll use a fixed path for rbenv)
RUN mkdir -p /home/user

# Tell rbenv explicitly where it lives (don't rely on $HOME during build)
ENV RBENV_ROOT=/home/user/.rbenv
ENV PATH="$RBENV_ROOT/bin:$RBENV_ROOT/shims:${PATH}"

# Clone rbenv and ruby-build
RUN git clone --depth 1 https://github.com/rbenv/rbenv.git "$RBENV_ROOT" && \
    git clone --depth 1 https://github.com/rbenv/ruby-build.git "$RBENV_ROOT/plugins/ruby-build"

# Optional: initialize rbenv for interactive shells
RUN echo 'eval "$('"$RBENV_ROOT"'/bin/rbenv init -)"' >> /root/.bashrc && \
    echo 'eval "$('"$RBENV_ROOT"'/bin/rbenv init -)"' >> /root/.profile

# Install Ruby versions and gems
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" \
    && rbenv install 3.4.2 && rbenv global 3.4.2 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.4.1 && rbenv shell 3.4.1 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.3.6 && rbenv shell 3.3.6 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.2.6 && rbenv shell 3.2.6 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.2.2 && rbenv shell 3.2.2 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash \
    && rbenv install 3.3.5 && rbenv shell 3.3.5 && rbenv exec gem install ruby-lsp sorbet && rbenv rehash

# Keep the original HOME placement from your Dockerfile
ENV HOME=/home/user

# Download and run dotnet install script
RUN curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh -o dotnet-install.sh \
    && chmod +x dotnet-install.sh \
    && ./dotnet-install.sh --channel 8.0 --install-dir /home/user/.dotnet \
    && ./dotnet-install.sh --channel 9.0 --install-dir /home/user/.dotnet \
    && rm dotnet-install.sh

# Add .NET to PATH
ENV PATH="${PATH}:/home/user/.dotnet"
ENV DOTNET_ROOT=/home/user/.dotnet

# Install csharp-ls globally
RUN dotnet tool install --global csharp-ls

# Add .NET tools to PATH
ENV PATH="${PATH}:/home/user/.dotnet/tools"

COPY Cargo.toml .

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN touch src/lib.rs
RUN cargo build --release

RUN cargo install ast-grep --locked
COPY ./src/ast_grep /usr/src/ast_grep

# Document that the container listens on port 4444
EXPOSE 4444

# Set environment variables for logging and backtrace
ENV RUST_LOG=debug,glob=info,ignore=info
ENV RUST_BACKTRACE=1
ENV RA_LOG="/tmp/rust-analyzer.log"

CMD /bin/bash -c "touch Cargo.toml && cargo build --release"
