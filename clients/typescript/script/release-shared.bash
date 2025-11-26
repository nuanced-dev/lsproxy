## Shared functions for lsp release scripts.

# Reads a JSON field from config/version.json
read_version_field() {
  local field="$1"
  node -e 'const fs=require("fs");const p=process.argv[1];const f=process.argv[2];process.stdout.write(JSON.parse(fs.readFileSync(p,"utf8"))[f]||"");' "$CONFIG_JSON" "$field"
}

read_lsp_version() {
  read_version_field "lsp-version"
}

read_lsp_proxy_version() {
  read_version_field "lsp-proxy-version"
}

read_lsp_wrapper_version() {
  read_version_field "lsp-wrapper-version"
}

read_lsp_watchdog_version() {
  read_version_field "lsp-watchdog-version"
}

read_language_container_version() {
  read_version_field "language-container-version"
}

# Validates all 5 required versions are present and valid semver
validate_all_versions() {
  local missing=()

  LSP_VERSION="$(read_lsp_version)"
  LSP_PROXY_VERSION="$(read_lsp_proxy_version)"
  LSP_WRAPPER_VERSION="$(read_lsp_wrapper_version)"
  LSP_WATCHDOG_VERSION="$(read_lsp_watchdog_version)"
  LANGUAGE_CONTAINER_VERSION="$(read_language_container_version)"

  if [[ -z "$LSP_VERSION" ]]; then
    missing+=("lsp-version")
  elif ! is_semver "$LSP_VERSION"; then
    die "lsp-version must be in x.y.z form (got: '$LSP_VERSION')"
  fi

  if [[ -z "$LSP_PROXY_VERSION" ]]; then
    missing+=("lsp-proxy-version")
  elif ! is_semver "$LSP_PROXY_VERSION"; then
    die "lsp-proxy-version must be in x.y.z form (got: '$LSP_PROXY_VERSION')"
  fi

  if [[ -z "$LSP_WRAPPER_VERSION" ]]; then
    missing+=("lsp-wrapper-version")
  elif ! is_semver "$LSP_WRAPPER_VERSION"; then
    die "lsp-wrapper-version must be in x.y.z form (got: '$LSP_WRAPPER_VERSION')"
  fi

  if [[ -z "$LSP_WATCHDOG_VERSION" ]]; then
    missing+=("lsp-watchdog-version")
  elif ! is_semver "$LSP_WATCHDOG_VERSION"; then
    die "lsp-watchdog-version must be in x.y.z form (got: '$LSP_WATCHDOG_VERSION')"
  fi

  if [[ -z "$LANGUAGE_CONTAINER_VERSION" ]]; then
    missing+=("language-container-version")
  elif ! is_semver "$LANGUAGE_CONTAINER_VERSION"; then
    die "language-container-version must be in x.y.z form (got: '$LANGUAGE_CONTAINER_VERSION')"
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    die "Missing required versions in $CONFIG_JSON: ${missing[*]}"
  fi
}
