#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_NAME="Kër Finance"
VERSION="$(node -p "require('${PROJECT_DIR}/package.json').version")"
ARCHITECTURE="$(uname -m)"
APP_PATH="${PROJECT_DIR}/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
DMG_DIR="${PROJECT_DIR}/src-tauri/target/release/bundle/dmg"
DMG_PATH="${DMG_DIR}/${APP_NAME}_${VERSION}_${ARCHITECTURE}.dmg"
STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ker-finance-dmg.XXXXXX")"

cleanup() {
  rm -rf "${STAGING_DIR}"
}
trap cleanup EXIT

cd "${PROJECT_DIR}"
npx tauri build --ci --bundles app

mkdir -p "${DMG_DIR}"
cp -R "${APP_PATH}" "${STAGING_DIR}/"
ln -s /Applications "${STAGING_DIR}/Applications"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${STAGING_DIR}" \
  -ov \
  -format UDZO \
  "${DMG_PATH}"

printf '\nApplication : %s\nDMG         : %s\n' "${APP_PATH}" "${DMG_PATH}"
