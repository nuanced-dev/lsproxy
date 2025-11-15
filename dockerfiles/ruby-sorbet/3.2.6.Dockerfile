# Ruby Sorbet 3.2.6 LSP server container
# Builds on top of the Ruby 3.2.6 image and adds sorbet gem

FROM nuanced-lsp-ruby-3.2.6:1.0.0

ENV DEBIAN_FRONTEND=noninteractive

# Install sorbet gem using the existing Ruby installation
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" && \
    rbenv exec gem install sorbet && \
    rbenv rehash

# Set language for lsp-wrapper configuration

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
RUN ln -s ${RBENV_ROOT}/shims/srb /usr/local/bin/srb && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true
ENV LSP_LANGUAGE="ruby-sorbet"

# Set workspace path
WORKDIR /mnt/workspace

# Use wrapper ENTRYPOINT (mounted from wrapper container)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "srb", "--lsp-arg=tc", "--lsp-arg=--lsp", "--lsp-arg=--disable-watchman"]
