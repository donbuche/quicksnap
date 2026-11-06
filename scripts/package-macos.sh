#!/usr/bin/env bash
# Builds QuickSnap.app and QuickSnap.dmg into dist/.
#   ./scripts/package-macos.sh
set -euo pipefail

APP_NAME="QuickSnap"
BIN_NAME="quicksnap"
BUNDLE_ID="com.joangaltes.quicksnap"

cd "$(dirname "$0")/.."
ROOT="$PWD"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"
DMG="$DIST/$APP_NAME-$VERSION.dmg"

echo "==> cargo build --release"
cargo build --release
BIN="$ROOT/target/release/$BIN_NAME"

echo "==> Building $APP_NAME.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"

# Icon: render PNGs from the app's own icon code, then pack them into .icns.
ICONSET="$(mktemp -d)/$APP_NAME.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  "$BIN" --write-icon "$ICONSET/icon_${s}x${s}.png" "$s"
  "$BIN" --write-icon "$ICONSET/icon_${s}x${s}@2x.png" "$((s * 2))"
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/$APP_NAME.icns"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>               <string>$APP_NAME</string>
  <key>CFBundleDisplayName</key>        <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>         <string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key>         <string>$BIN_NAME</string>
  <key>CFBundleIconFile</key>           <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>        <string>APPL</string>
  <key>CFBundleVersion</key>            <string>$VERSION</string>
  <key>CFBundleShortVersionString</key> <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>     <string>11.0</string>
  <key>NSHighResolutionCapable</key>    <true/>
  <key>LSApplicationCategoryType</key>  <string>public.app-category.productivity</string>
</dict>
</plist>
EOF

# Ad-hoc signature: free, no Apple account. Gives the bundle a stable identity
# so the Screen Recording permission sticks to QuickSnap.
echo "==> Ad-hoc signing"
codesign --force --deep --sign - "$APP"

echo "==> Building $(basename "$DMG")"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE" "$(dirname "$ICONSET")"

echo
echo "Done:"
echo "  $APP"
echo "  $DMG"
