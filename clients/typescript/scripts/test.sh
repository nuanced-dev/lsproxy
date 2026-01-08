#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $0 [--fail-fast] [--languages=LANG...] [--proxy-image=<name:tag>] [--record-fixtures] [--symbol-delay=<secs>] [--timeout=<secs>] [--watchdog-image=<name:tag>] [--workers=N] [--workspace-delay=<secs>] [--wrapper-image=<name:tag>] [-- VITEST_ARGS...]"
}

help() {
    echo "Test runner for the Vitest test suite"
    echo ""
    echo "Usage: $0 [OPTIONS...] [-- VITEST_ARGS...]"
    echo ""
    echo "Requirements:"
    echo "  - Node.js 18+"
    echo ""
    echo "Options:"
    echo "  --fail-fast                    Stop on first failure"
    echo "  --languages=LANG...            Languages to include in the test (default: all)"
    echo "  --proxy-image=<name:tag>       Docker image override"
    echo "  --record-fixtures              Record fixtures"
    echo "  --symbol-delay=<secs>          Delay before symbol scenario tests (default: 0)"
    echo "  --timeout=<secs>               Per-command timeout (default: 120)"
    echo "  --watchdog-image=<name:tag>    Docker image override for watchdog"
    echo "  --workers=N                    Number of parallel workers (default: use all CPU cores)"
    echo "  --workspace-delay=<secs>       Delay before workspace scenario tests (default: 0)"
    echo "  --wrapper-image=<name:tag>     Docker image override for wrapper"
    echo "  -h, --help                     Show this help message"
    echo ""
    echo "Any additional args are passed through to Vitest."
    echo ""
    echo "Examples:"
    echo "  $0"
    echo "  $0 --fail-fast"
    echo "  $0 --proxy-image=ghcr.io/nuanced-dev/nuanced-lsp-proxy:latest"
    echo "  $0 --record-fixtures"
    echo "  $0 --timeout=300"
    echo "  $0 --workers=8"
    echo "  $0 -- tests/specs/workspace-symbols.spec.ts"
    echo "  $0 -- --testNamePattern 'find definitions'"
}

FAIL_FAST=                   # if true, stop on first failure.
NUANCED_LANGUAGES=           # languages to run tests for
PROXY_IMAGE=                 # allows overriding the default nuanced-lsp-proxy image used in tests.
RECORD_FIXTURES=             # if true, record fixtures instead of comparing.
SYMBOL_SCENARIO_DELAY=       # delay (in seconds) before running symbol scenario tests.
NUANCED_LSP_TIMEOUT=         # number of seconds each test case is allowed to run.
WATCHDOG_IMAGE=              # allows overriding the default nuanced-lsp-watchdog image used in tests.
WORKERS=                     # if empty, vitest uses all CPU cores.
WORKSPACE_SCENARIO_DELAY=    # delay (in seconds) before running workspace scenario
WRAPPER_IMAGE=               # allows overriding the default nuanced-lsp-wrapper image used in tests.

while [[ $# -gt 0 ]]; do
    arg="$1"
    case $arg in
        --fail-fast)
            export FAIL_FAST=1
            ;;
        --languages=*)
            export NUANCED_LANGUAGES="${arg#*=}"
            ;;
        --proxy-image=*)
            export PROXY_IMAGE="${arg#*=}"
            ;;
        --record-fixtures)
            export RECORD_FIXTURES=1
            ;;
        --symbol-delay=*)
            export SYMBOL_SCENARIO_DELAY="${arg#*=}"
            ;;
        --timeout=*)
            export NUANCED_LSP_TIMEOUT="${arg#*=}"
            ;;
        --watchdog-image=*)
            export WATCHDOG_IMAGE="${arg#*=}"
            ;;
        --workers=*)
            export WORKERS="${arg#*=}"
            ;;
        --workspace-delay=*)
            export WORKSPACE_SCENARIO_DELAY="${arg#*=}"
            ;;
        --wrapper-image=*)
            export WRAPPER_IMAGE="${arg#*=}"
            ;;
        -h|--help)
            help
            exit 0
            ;;
        --)
            shift
            break # only passthrough arguments remain
            ;;
        *)
            echo -e "Unknown option: $arg"
            usage
            exit 1
            ;;
    esac
    shift
done

npm run build

exec npx vitest run "$@"
