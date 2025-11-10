# Ruby Sorbet 3.3.1 LSP server container
# Builds on top of the Ruby 3.3.1 image and adds sorbet gem

FROM lsproxy-ruby-3.3.1:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install sorbet gem using the existing Ruby installation
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" && \
    rbenv exec gem install sorbet && \
    rbenv rehash

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="ruby-sorbet"

# Set workspace path
WORKDIR /mnt/workspace

# Use wrapper ENTRYPOINT (mounted from wrapper container)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "srb", "--lsp-arg=tc", "--lsp-arg=--lsp", "--lsp-arg=--disable-watchman"]
