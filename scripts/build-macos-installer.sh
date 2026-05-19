#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="OpenTokenMonitor"
VERSION="$(node -p "require('${ROOT_DIR}/package.json').version")"
APP_BUNDLE="${ROOT_DIR}/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
DMG_DIR="${ROOT_DIR}/src-tauri/target/release/bundle/dmg"
DMG_PATH="${DMG_DIR}/${APP_NAME}_${VERSION}_$(uname -m).dmg"
STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/${APP_NAME}.dmg.XXXXXX")"

cleanup() {
  rm -rf "${STAGING_DIR}"
}

trap cleanup EXIT

if [[ ! -d "${APP_BUNDLE}" ]]; then
  echo "Expected app bundle not found: ${APP_BUNDLE}" >&2
  exit 1
fi

# `tauri build` notarizes + staples the .app when Apple credentials are set.
# Surface that state so a release build never silently ships unstapled.
if xcrun stapler validate "${APP_BUNDLE}" >/dev/null 2>&1; then
  echo "App bundle is notarized + stapled."
else
  echo "Note: app bundle is not stapled (ad-hoc/unsigned build, or notarization skipped)."
fi

mkdir -p "${DMG_DIR}"
rm -f "${DMG_PATH}"

cp -R "${APP_BUNDLE}" "${STAGING_DIR}/"
ln -s /Applications "${STAGING_DIR}/Applications"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${STAGING_DIR}" \
  -ov \
  -format UDZO \
  "${DMG_PATH}"

echo "Created macOS installer: ${DMG_PATH}"

# The .app inside is already notarized+stapled by `tauri build`. This DMG is
# built outside Tauri, so sign + notarize + staple the wrapper here too, so the
# downloaded installer itself is trusted by Gatekeeper. All steps are gated on
# Apple credentials — local/dev builds still produce a working unsigned DMG.
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  echo "Signing DMG with ${APPLE_SIGNING_IDENTITY}"
  codesign --force --sign "${APPLE_SIGNING_IDENTITY}" "${DMG_PATH}"
fi

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
  echo "Notarizing DMG (this can take a few minutes)"
  xcrun notarytool submit "${DMG_PATH}" \
    --apple-id "${APPLE_ID}" \
    --password "${APPLE_PASSWORD}" \
    --team-id "${APPLE_TEAM_ID}" \
    --wait
  xcrun stapler staple "${DMG_PATH}"
  echo "DMG notarized and stapled."
else
  echo "Apple notarization credentials not set; DMG left un-notarized (local/dev build)."
fi
