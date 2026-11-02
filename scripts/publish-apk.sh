#!/usr/bin/env bash
# Publish one Hearth release. One command, no manual steps.
# Builds the release APK, signs it, writes and signs the manifest,
# and serves both from the update folder. Fails closed everywhere.
#
# Needs (owner runs on the host, outside any sandbox):
#   ~/.config/lifeos/hearth-release.keystore  (mode 600, made once with keytool)
#   ~/.config/lifeos/update-signing.key       (mode 600, Ed25519, made once with openssl)
#   ~/.config/lifeos/.storepass               (mode 600, holds the keystore password)
# HEARTH_KEYSTORE_PASSWORD in env overrides the file. Key password defaults
# to the same value. Never prints secrets. Prints code, size, hash, and
# fingerprint only.
set -euo pipefail
cd "$(dirname "$0")/.."

NOTES="${1:-Stability fixes.}"
KEYDIR="$HOME/.config/lifeos"
KEYSTORE="$KEYDIR/hearth-release.keystore"
SIGNKEY="$KEYDIR/update-signing.key"
UPDATE_DIR="${UPDATE_DIR:-$HOME/hearth/updates}"
CODE_FILE="web/src-tauri/version-code.txt"
API_BASE="${PUBLIC_HEARTH_API_BASE:-https://example.ts.net:8447}"

test -f "$KEYSTORE" || { echo "PUBLISH FAIL: $KEYSTORE missing (see web/TAURI.md)"; exit 1; }
test -f "$SIGNKEY" || { echo "PUBLISH FAIL: $SIGNKEY missing (see web/TAURI.md)"; exit 1; }
if [ -z "${HEARTH_KEYSTORE_PASSWORD:-}" ] && [ -f "$KEYDIR/.storepass" ]; then
  HEARTH_KEYSTORE_PASSWORD="$(cat "$KEYDIR/.storepass")"
fi
test -n "${HEARTH_KEYSTORE_PASSWORD:-}" || { echo "PUBLISH FAIL: export HEARTH_KEYSTORE_PASSWORD first"; exit 1; }
command -v podman >/dev/null || { echo "PUBLISH FAIL: podman missing"; exit 1; }
command -v python3 >/dev/null || { echo "PUBLISH FAIL: python3 missing"; exit 1; }

# PRE-PUBLISH GATE. The phone link must work before any build.
# The bridge check plus a live needs-you read. No release if either fails.
"$(dirname "$0")/agent-bridge.sh" check || { echo "PUBLISH FAIL: bridge check failed"; exit 1; }
test -n "${HEARTH_TOKEN:-}" || { echo "PUBLISH FAIL: export HEARTH_TOKEN first"; exit 1; }
GATE_CODE="$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 -H "Authorization: Bearer $HEARTH_TOKEN" "$API_BASE/v1/watch/needs-you" || true)"
test "$GATE_CODE" = "200" || { echo "PUBLISH FAIL: needs-you check got ${GATE_CODE:-none}, want 200"; exit 1; }
echo "Gate passed: bridge and needs-you answer."

CODE=$(($(cat "$CODE_FILE") + 1))
ALIAS="${HEARTH_KEY_ALIAS:-hearth}"
KEYPASS="${HEARTH_KEY_PASSWORD:-$HEARTH_KEYSTORE_PASSWORD}"

# Public key bytes for the app pin: raw 32 bytes, base64.
PUBKEY_B64="$(openssl pkey -in "$SIGNKEY" -pubout -outform DER 2>/dev/null | tail -c 32 | base64 -w0)"
test -n "$PUBKEY_B64" || { echo "PUBLISH FAIL: cannot read signing key"; exit 1; }

echo "== publish: release code $CODE =="
podman run --rm --user 0:0 \
  -v "$PWD:/work:Z" \
  -v "$KEYDIR:/keys:ro,z" \
  -w /work/web \
  -e "PUBLIC_HEARTH_API_BASE=$API_BASE" \
  -e "APP_VERSION_CODE=$CODE" \
  -e "PUBLIC_APP_VERSION_CODE=$CODE" \
  -e "UPDATE_PUBKEY_B64=$PUBKEY_B64" \
  -e "HEARTH_KEYSTORE_PATH=/keys/hearth-release.keystore" \
  -e "HEARTH_KEYSTORE_PASSWORD=$HEARTH_KEYSTORE_PASSWORD" \
  -e "HEARTH_KEY_ALIAS=$ALIAS" \
  -e "HEARTH_KEY_PASSWORD=$KEYPASS" \
  localhost/hearth-android-build:latest \
  sh -c 'set -e
    curl -fsSL -o /tmp/node.tar.xz https://nodejs.org/dist/v22.20.0/node-v22.20.0-linux-x64.tar.xz
    mkdir -p /opt/node && tar -xJf /tmp/node.tar.xz -C /opt/node --strip-components=1
    export PATH=/opt/node/bin:$PATH
    cargo tauri android build --apk --config "{\"bundle\":{\"android\":{\"versionCode\":$APP_VERSION_CODE}}}"'

APK="web/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk"
test -f "$APK" || { echo "PUBLISH FAIL: no APK at $APK"; exit 1; }

# Verify inside the image (has the SDK tools): code, signature, key match.
# Single-quoted script; values cross through env so nothing needs escaping.
GOT_CERTS="$(podman run --rm --user 0:0 -v "$PWD:/work:Z" -w /work \
  -e "WANT_CODE=$CODE" -e "APK_PATH=/work/$APK" \
  localhost/hearth-android-build:latest sh -c '
  set -e
  BT=$(echo /opt/android-sdk/build-tools/*/ | tr " " "\n" | sort -V | tail -1)
  CODE_OUT=$($BT/aapt2 dump badging "$APK_PATH" | grep -oP "versionCode=\x27\K[0-9]+" | head -1)
  test "$WANT_CODE" = "$CODE_OUT" || { echo "PUBLISH FAIL: APK code $CODE_OUT, want $WANT_CODE"; exit 1; }
  $BT/apksigner verify --print-certs "$APK_PATH"
')"
test -n "$GOT_CERTS" || { echo "PUBLISH FAIL: signature verify failed"; exit 1; }
norm() { echo "$1" | tr -cd '0-9A-Fa-f' | tr 'a-f' 'A-F'; }
GOT_FP="$(echo "$GOT_CERTS" | grep -i "SHA-256 digest" | head -1 | sed 's/.*digest: *//')"
WANT_FP="$(keytool -list -v -keystore "$KEYSTORE" -alias "$ALIAS" -storepass "$HEARTH_KEYSTORE_PASSWORD" 2>/dev/null | grep -i "SHA256:" | head -1 | sed 's/.*SHA256: *//')"
test -n "$GOT_FP" || { echo "PUBLISH FAIL: no signer cert in APK"; exit 1; }
test "$(norm "$GOT_FP")" = "$(norm "$WANT_FP")" || { echo "PUBLISH FAIL: APK signed by the wrong key"; exit 1; }
echo "keystore cert: $WANT_FP"

SHA="$(sha256sum "$APK" | cut -d' ' -f1)"
SIZE="$(stat -c%s "$APK")"
VERSION="$(grep -m1 '^version' web/src-tauri/Cargo.toml | cut -d'"' -f2)"

# Canonical manifest bytes: sorted keys, compact, no trailing newline.
# Notes pass through env so shell chars in them stay literal.
MANIFEST="$(NOTES="$NOTES" VERSION="$VERSION" CODE="$CODE" SIZE="$SIZE" SHA="$SHA" python3 -c "
import json, os
print(json.dumps({'version': os.environ['VERSION'], 'versionCode': int(os.environ['CODE']), 'apk_url': '/v1/app/apk', 'apk_size': int(os.environ['SIZE']), 'sha256': os.environ['SHA'], 'notes': os.environ['NOTES']}, sort_keys=True, separators=(',', ':')), end='')
")"
mkdir -p "$UPDATE_DIR"
printf '%s' "$MANIFEST" > "$UPDATE_DIR/manifest.json"
openssl pkeyutl -sign -inkey "$SIGNKEY" -rawin -in "$UPDATE_DIR/manifest.json" -out "$UPDATE_DIR/manifest.sig"
test "$(stat -c%s "$UPDATE_DIR/manifest.sig")" = "64" || { echo "PUBLISH FAIL: bad signature size"; exit 1; }

# Keep the last two APKs. Then consume the code.
test -f "$UPDATE_DIR/hearth.apk" && mv -f "$UPDATE_DIR/hearth.apk" "$UPDATE_DIR/hearth-prev.apk" || true
cp -f "$APK" "$UPDATE_DIR/hearth.apk"
echo "$CODE" > "$CODE_FILE"

echo "PUBLISH OK: code $CODE, $SIZE bytes"
echo "sha256: $SHA"
echo "served from: $UPDATE_DIR (manifest.json, manifest.sig, hearth.apk)"
