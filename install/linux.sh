#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

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
      echo "  default  : install BREOM_HOME into $HOME/.breom"
      echo "  --system : install BREOM_HOME into /usr/local/breom"
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

DEFAULT_BREOM_HOME_PATH="$HOME/.breom"
if [[ "$USE_SYSTEM_HOME" -eq 1 ]]; then
  DEFAULT_BREOM_HOME_PATH="/usr/local/breom"
fi
BREOM_HOME_PATH="${BREOM_HOME_PATH:-$DEFAULT_BREOM_HOME_PATH}"

BREOM_VERSION="$(awk -F'"' '/^version = "/ { print $2; exit }' "$REPO_ROOT/Cargo.toml")"
if [[ -z "$BREOM_VERSION" ]]; then
  echo "error: failed to read version from $REPO_ROOT/Cargo.toml"
  exit 1
fi

VERSIONED_ROOT_PATH="$BREOM_HOME_PATH/$BREOM_VERSION"
VERSIONED_STD_PATH="$VERSIONED_ROOT_PATH/src"
BREOM_BIN_PATH="$VERSIONED_ROOT_PATH/bin"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is not installed. Install Rust from https://rustup.rs first."
  exit 1
fi

echo "[1/4] Installing breom CLI via cargo..."
cargo install --path "$REPO_ROOT" --locked --force --root "$VERSIONED_ROOT_PATH"

echo "[2/4] Preparing BREOM_HOME at $BREOM_HOME_PATH"
mkdir -p "$VERSIONED_STD_PATH"

if [[ -d "$REPO_ROOT/std" ]]; then
  if command -v rsync >/dev/null 2>&1; then
    rsync -a --delete "$REPO_ROOT/std/" "$VERSIONED_STD_PATH/"
  else
    rm -rf "$VERSIONED_STD_PATH"
    mkdir -p "$VERSIONED_STD_PATH"
    cp -R "$REPO_ROOT/std/." "$VERSIONED_STD_PATH/"
  fi
fi

echo "[3/4] Configuring BREOM_HOME=$BREOM_HOME_PATH"

PROFILE_FILES=(
  "$HOME/.zshrc"
  "$HOME/.zprofile"
  "$HOME/.zshenv"
  "$HOME/.zlogin"
  "$HOME/.bashrc"
  "$HOME/.bash_profile"
  "$HOME/.bash_login"
  "$HOME/.profile"
)
EXPORT_LINE="export BREOM_HOME=\"$BREOM_HOME_PATH\""
PATH_EXPORT_LINE="export PATH=\"$BREOM_BIN_PATH:\$PATH\" # breom-bin"

for profile in "${PROFILE_FILES[@]}"; do
  if [[ ! -f "$profile" ]]; then
    continue
  fi

  if grep -F "export BREOM_HOME=" "$profile" >/dev/null 2>&1; then
    tmp_file="$(mktemp)"
    awk -v line="$EXPORT_LINE" '
      BEGIN { replaced = 0 }
      /^export BREOM_HOME=/ {
        if (!replaced) {
          print line
          replaced = 1
        }
        next
      }
      { print }
      END {
        if (!replaced) print line
      }
    ' "$profile" > "$tmp_file"
    mv "$tmp_file" "$profile"
  else
    printf "\n%s\n" "$EXPORT_LINE" >> "$profile"
  fi

  if grep -F "# breom-bin" "$profile" >/dev/null 2>&1; then
    tmp_file="$(mktemp)"
    awk -v line="$PATH_EXPORT_LINE" '
      BEGIN { replaced = 0 }
      /# breom-bin$/ {
        if (!replaced) {
          print line
          replaced = 1
        }
        next
      }
      { print }
      END {
        if (!replaced) print line
      }
    ' "$profile" > "$tmp_file"
    mv "$tmp_file" "$profile"
  else
    printf "%s\n" "$PATH_EXPORT_LINE" >> "$profile"
  fi
done

FISH_CONFIG="$HOME/.config/fish/config.fish"
if [[ -f "$FISH_CONFIG" ]]; then
  if grep -F "set -gx BREOM_HOME" "$FISH_CONFIG" >/dev/null 2>&1; then
    tmp_file="$(mktemp)"
    awk -v line="set -gx BREOM_HOME \"$BREOM_HOME_PATH\"" '
      BEGIN { replaced = 0 }
      /^set -gx BREOM_HOME / {
        if (!replaced) {
          print line
          replaced = 1
        }
        next
      }
      { print }
      END {
        if (!replaced) print line
      }
    ' "$FISH_CONFIG" > "$tmp_file"
    mv "$tmp_file" "$FISH_CONFIG"
  else
    printf "\nset -gx BREOM_HOME \"%s\"\n" "$BREOM_HOME_PATH" >> "$FISH_CONFIG"
  fi

  if grep -F "# breom-bin" "$FISH_CONFIG" >/dev/null 2>&1; then
    tmp_file="$(mktemp)"
    awk -v line="fish_add_path -m \"$BREOM_BIN_PATH\" # breom-bin" '
      BEGIN { replaced = 0 }
      /# breom-bin$/ {
        if (!replaced) {
          print line
          replaced = 1
        }
        next
      }
      { print }
      END {
        if (!replaced) print line
      }
    ' "$FISH_CONFIG" > "$tmp_file"
    mv "$tmp_file" "$FISH_CONFIG"
  else
    printf "fish_add_path -m \"%s\" # breom-bin\n" "$BREOM_BIN_PATH" >> "$FISH_CONFIG"
  fi
fi

export BREOM_HOME="$BREOM_HOME_PATH"
export PATH="$BREOM_BIN_PATH:$PATH"

echo "[4/4] Done."
echo
echo "BREOM_HOME is now set to: $BREOM_HOME"
echo "std installed to: $VERSIONED_STD_PATH"
echo "breom binary path: $BREOM_BIN_PATH"
echo "Open a new shell or run: source ~/.zshrc (or your shell profile)."
echo "Verify with: breom --help"
