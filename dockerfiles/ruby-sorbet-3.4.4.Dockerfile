# Ruby Sorbet 3.4.4 LSP server container
# Builds on top of the Ruby 3.4.4 image and adds sorbet gem
#
# To build for a different Ruby version:
#   1. Ensure ruby-X.Y.Z.Dockerfile exists and has been built
#   2. Copy this file to ruby-sorbet-X.Y.Z.Dockerfile
#   3. Update the FROM line below to use lsproxy-ruby-X.Y.Z:latest
#   4. Build: docker build -f dockerfiles/ruby-sorbet-X.Y.Z.Dockerfile -t lsproxy-ruby-sorbet-X.Y.Z:latest .

FROM lsproxy-ruby-3.4.4:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install sorbet gem using the existing Ruby installation
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" && \
    rbenv exec gem install sorbet && \
    rbenv rehash

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "srb", "--lsp-arg=tc", "--lsp-arg=--lsp", "--lsp-arg=--disable-watchman"]
