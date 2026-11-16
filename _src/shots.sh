#!/bin/bash
# Renders the app screenshots used by the site, for every language and theme.
#
#   _src/shots.sh [path/to/quicksnap]
#
# The binary defaults to target/release/quicksnap (build it on develop with
# `cargo build --release`; target/ is ignored, so it survives `git switch`).
# Writes assets/shots/<lang>/<view>-<theme>.jpg
set -euo pipefail

cd "$(dirname "$0")/.."
BIN="${1:-target/release/quicksnap}"
[ -x "$BIN" ] || { echo "QuickSnap binary not found: $BIN" >&2; exit 1; }

LANGS="en es ca gl eu fr de it pt pt-br nl pl ja zh ko"
VIEWS="home editor prefs-general prefs-hotkeys prefs-appearance"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for lang in $LANGS; do
  out="assets/shots/$lang"
  mkdir -p "$out"
  for view in $VIEWS; do
    for theme in dark light; do
      png="$TMP/$view-$theme.png"
      if [ "$view" = editor ]; then
        QUICKSNAP_LANG=$lang QUICKSNAP_THEME=$theme QUICKSNAP_WINDOW=1000x645 \
          "$BIN" --screenshot "$png" >/dev/null 2>&1
      else
        QUICKSNAP_VIEW=$view QUICKSNAP_LANG=$lang QUICKSNAP_THEME=$theme QUICKSNAP_WINDOW=1000x645 \
          "$BIN" --screenshot "$png" >/dev/null 2>&1
      fi
      case "$view" in
        home|editor) size=1600 ;;
        *) size=900 ;;
      esac
      sips -Z "$size" -s format jpeg -s formatOptions 80 "$png" --out "$out/$view-$theme.jpg" >/dev/null
    done
  done
  echo "  $lang done"
done
