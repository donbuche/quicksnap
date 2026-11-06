#!/usr/bin/env bash
# One-time setup: creates a self-signed code-signing certificate in your login
# keychain so every QuickSnap build carries the same signature. macOS then
# keeps the Screen Recording permission across rebuilds instead of asking again.
#   ./scripts/setup-signing.sh
set -euo pipefail

NAME="QuickSnap Local Signing"
BUNDLE_ID="com.joangaltes.quicksnap"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

if security find-certificate -c "$NAME" "$KEYCHAIN" >/dev/null 2>&1; then
  echo "Certificate \"$NAME\" already exists; nothing to do."
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
EOF

echo "==> Creating certificate \"$NAME\" (valid 10 years)"
/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$TMP/key.pem" -out "$TMP/cert.pem" -config "$TMP/cert.cnf" 2>/dev/null

PASS="$(/usr/bin/openssl rand -hex 12)"
# OpenSSL 3 needs -legacy for a .p12 the macOS keychain can read.
LEGACY=""
/usr/bin/openssl version | grep -q "^OpenSSL 3" && LEGACY="-legacy"
/usr/bin/openssl pkcs12 -export $LEGACY -inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
  -name "$NAME" -out "$TMP/id.p12" -passout "pass:$PASS"

echo "==> Importing into the login keychain (codesign may use the key)"
security import "$TMP/id.p12" -k "$KEYCHAIN" -P "$PASS" -T /usr/bin/codesign

# The old grant belongs to the previous (ad-hoc) signatures; clear it so the
# next capture asks once and the answer then sticks.
echo "==> Resetting QuickSnap's old Screen Recording entry"
tccutil reset ScreenCapture "$BUNDLE_ID" || true

echo
echo "Done. Now run ./scripts/install-local.sh and allow Screen Recording once."
