#!/usr/bin/env bash
# Builds QuickSnap.app, replaces the copy in /Applications and launches it.
# For day-to-day testing; no .dmg involved.
#   ./scripts/install-local.sh
set -euo pipefail

APP_NAME="QuickSnap"
BIN_NAME="quicksnap"
DEST="/Applications/$APP_NAME.app"

cd "$(dirname "$0")/.."

# Sign with the fixed certificate from setup-signing.sh when it exists, so the
# Screen Recording permission survives reinstalls.
CERT="QuickSnap Local Signing"
if [ -z "${SIGN_IDENTITY:-}" ] && security find-certificate -c "$CERT" >/dev/null 2>&1; then
  export SIGN_IDENTITY="$CERT"
fi
if [ -z "${SIGN_IDENTITY:-}" ]; then
  echo "Note: no signing certificate; run ./scripts/setup-signing.sh once so"
  echo "      macOS stops asking for Screen Recording after every install."
fi
./scripts/package-macos.sh --no-dmg

echo "==> Quitting the running $APP_NAME"
if pgrep -x "$BIN_NAME" >/dev/null; then
  pkill -x "$BIN_NAME" || true
  for _ in $(seq 1 50); do pgrep -x "$BIN_NAME" >/dev/null || break; sleep 0.1; done
  pkill -9 -x "$BIN_NAME" 2>/dev/null || true
fi

echo "==> Installing to $DEST"
rm -rf "$DEST"
ditto "dist/$APP_NAME.app" "$DEST"

echo "==> Launching"
open "$DEST"
for _ in $(seq 1 50); do pgrep -x "$BIN_NAME" >/dev/null && break; sleep 0.1; done
if pgrep -x "$BIN_NAME" >/dev/null; then
  echo "Running: $DEST (pid $(pgrep -x "$BIN_NAME"))"
else
  echo "Error: $APP_NAME did not start" >&2
  exit 1
fi
