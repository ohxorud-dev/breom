#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

USE_SYSTEM_HOME=0
for arg in "$@"; do
  case "$arg" in
    --user)
      USE_SYSTEM_HOME=0
      ;;
    --system)
      USE_SYSTEM_HOME=1
      ;;
    -h|--help)
      echo "Usage: $0 [--user|--system]"
      echo "  default  : install BREOM_HOME into $HOME/Library/Application Support/Breom"
      echo "  --system : install BREOM_HOME into /Library/Breom"
      echo "  BREOM_HOME_PATH env var overrides both defaults"
      exit 0
      ;;
    *)
      echo "error: unknown argument '$arg'"
      echo "Try: $0 --help"
      exit 1
      ;;
  esac
done

DEFAULT_BREOM_HOME_PATH="$HOME/Library/Application Support/Breom"
if [[ "$USE_SYSTEM_HOME" -eq 1 ]]; then
  DEFAULT_BREOM_HOME_PATH="/Library/Breom"
fi
export BREOM_HOME_PATH="${BREOM_HOME_PATH:-$DEFAULT_BREOM_HOME_PATH}"

REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
BREOM_VERSION="$(awk -F'"' '/^version = "/ { print $2; exit }' "$REPO_ROOT/Cargo.toml")"
if [[ -z "$BREOM_VERSION" ]]; then
  echo "error: failed to read version from $REPO_ROOT/Cargo.toml"
  exit 1
fi
BREOM_BIN_PATH="$BREOM_HOME_PATH/$BREOM_VERSION/bin"

"$SCRIPT_DIR/linux.sh" "$@"

if command -v launchctl >/dev/null 2>&1; then
  launchctl setenv BREOM_HOME "$BREOM_HOME_PATH" || true

  current_path="$(launchctl getenv PATH || true)"
  if [[ -z "$current_path" ]]; then
    current_path="$PATH"
  fi

  case ":$current_path:" in
    *":$BREOM_BIN_PATH:"*)
      ;;
    *)
      launchctl setenv PATH "$BREOM_BIN_PATH:$current_path" || true
      ;;
  esac
fi

echo
echo "macOS setup complete."
echo "If JetBrains/VSCode was already open, restart the app to pick up BREOM_HOME and PATH."
