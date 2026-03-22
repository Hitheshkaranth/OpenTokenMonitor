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
