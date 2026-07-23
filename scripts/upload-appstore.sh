#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG_PATH="${1:-$ROOT_DIR/dist/app-store/pH7Console.pkg}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "App Store uploads must run on macOS." >&2
  exit 1
fi

: "${APPLE_API_KEY_ID:?Set APPLE_API_KEY_ID to the App Store Connect API key ID.}"
: "${APPLE_API_ISSUER:?Set APPLE_API_ISSUER to the App Store Connect issuer ID.}"
: "${APPLE_API_KEY_PATH:?Set APPLE_API_KEY_PATH to the App Store Connect private key.}"

if [[ ! -f "$APPLE_API_KEY_PATH" ]]; then
  echo "App Store Connect private key not found: $APPLE_API_KEY_PATH" >&2
  exit 1
fi

if [[ ! -f "$PKG_PATH" ]]; then
  echo "Package not found: $PKG_PATH" >&2
  exit 1
fi

cd "$ROOT_DIR"
APP_PACKAGE_PATH="$PKG_PATH" fastlane mac upload_build_via_api
