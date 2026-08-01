#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAURI_DIR="$ROOT_DIR/src-tauri"
APP_IDENTIFIER="com.efficienttools.ph7console"
HELPER_IDENTIFIER="com.efficienttools.ph7console.llama-server"
MINIMUM_RELEASE_BUILD_NUMBER=5
EXPECTED_MINIMUM_SYSTEM_VERSION="13.3"
EXPECTED_MODEL_SHA256="cc324af070c2ecbfd324a30884d2f951a7ff756aba85cb811a6ec436933bb046"
EXPECTED_MODEL_SIZE="1117320768"
ENTITLEMENTS="$TAURI_DIR/Entitlements.plist"
HELPER_ENTITLEMENTS="$TAURI_DIR/HelperEntitlements.plist"
EMBEDDED_PROFILE="$TAURI_DIR/embedded.provisionprofile"
GENERATED_CONFIG="$TAURI_DIR/tauri.appstore.generated.conf.json"
APP_PATH="$TAURI_DIR/target/universal-apple-darwin/release/bundle/macos/pH7Console.app"
HELPER_INPUT="$TAURI_DIR/binaries/llama-server-universal-apple-darwin"
MODEL_INPUT="$TAURI_DIR/resources/models/terminal-assistant.gguf"
MODEL_RESOURCE_RELATIVE="resources/models/terminal-assistant.gguf"
NOTICE_INPUT="$TAURI_DIR/resources/models/NOTICE.md"
QWEN_LICENSE_INPUT="$TAURI_DIR/resources/models/LICENSE-QWEN"
LLAMA_CPP_LICENSE_INPUT="$TAURI_DIR/resources/models/LICENSE-LLAMA-CPP"
CPP_HTTPLIB_LICENSE_INPUT="$TAURI_DIR/resources/models/LICENSE-CPP-HTTPLIB"
NLOHMANN_JSON_LICENSE_INPUT="$TAURI_DIR/resources/models/LICENSE-NLOHMANN-JSON"
CLI_INPUT="$ROOT_DIR/scripts/ph7"
OUTPUT_DIR="$ROOT_DIR/dist/app-store"
PKG_PATH="$OUTPUT_DIR/pH7Console.pkg"
APPLE_TIMESTAMP_URL="http://timestamp.apple.com/ts01"

fail() {
  echo "$*" >&2
  exit 1
}

require_regular_file() {
  local path="$1"
  local description="$2"

  [[ -f "$path" ]] || fail "$description not found: $path"
  [[ ! -L "$path" ]] || fail "$description must not be a symbolic link: $path"
  [[ -s "$path" ]] || fail "$description is empty: $path"
}

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

verify_exact_model() {
  local path="$1"
  local actual_size
  local actual_sha256

  require_regular_file "$path" "Pinned Qwen GGUF"
  actual_size="$(stat -f %z "$path")"
  [[ "$actual_size" == "$EXPECTED_MODEL_SIZE" ]] ||
    fail "Pinned Qwen GGUF has size $actual_size; expected $EXPECTED_MODEL_SIZE bytes: $path"
  actual_sha256="$(sha256_file "$path")"
  [[ "$actual_sha256" == "$EXPECTED_MODEL_SHA256" ]] ||
    fail "Pinned Qwen GGUF SHA-256 is $actual_sha256; expected $EXPECTED_MODEL_SHA256: $path"
}

reject_release_xattrs() {
  local path="$1"
  local attribute

  for attribute in com.apple.quarantine com.apple.ResourceFork com.apple.FinderInfo; do
    if xattr -p "$attribute" "$path" >/dev/null 2>&1; then
      fail "Release input has forbidden extended attribute '$attribute': $path"
    fi
  done
}

verify_exact_universal_binary() {
  local executable_path="$1"
  local architecture
  local has_arm64=0
  local has_x86_64=0
  local architecture_count=0

  require_regular_file "$executable_path" "Universal executable"
  [[ -x "$executable_path" ]] || fail "Release executable is not executable: $executable_path"
  lipo "$executable_path" -verify_arch arm64 x86_64 >/dev/null ||
    fail "Executable is missing an arm64 or x86_64 slice: $executable_path"

  for architecture in $(lipo -archs "$executable_path"); do
    architecture_count=$((architecture_count + 1))
    case "$architecture" in
      arm64) has_arm64=1 ;;
      x86_64) has_x86_64=1 ;;
      *) fail "Unexpected architecture '$architecture' in $executable_path" ;;
    esac
  done

  [[ "$architecture_count" -eq 2 && "$has_arm64" -eq 1 && "$has_x86_64" -eq 1 ]] ||
    fail "Executable must contain exactly arm64 and x86_64 slices: $executable_path"
}

version_number() {
  local version="$1"
  local major
  local minor
  local patch

  [[ "$version" =~ ^([0-9]+)(\.([0-9]+))?(\.([0-9]+))?$ ]] || return 1
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[3]:-0}"
  patch="${BASH_REMATCH[5]:-0}"
  printf '%d\n' "$((major * 1000000 + minor * 1000 + patch))"
}

minimum_macos_for_architecture() {
  local executable_path="$1"
  local architecture="$2"

  otool -arch "$architecture" -l "$executable_path" | awk '
    $1 == "cmd" && $2 == "LC_BUILD_VERSION" { command = "build"; next }
    $1 == "cmd" && $2 == "LC_VERSION_MIN_MACOSX" { command = "legacy"; next }
    command == "build" && $1 == "minos" { print $2; exit }
    command == "legacy" && $1 == "version" { print $2; exit }
  '
}

verify_deployment_target() {
  local executable_path="$1"
  local architecture
  local actual_version
  local actual_number
  local expected_number

  expected_number="$(version_number "$EXPECTED_MINIMUM_SYSTEM_VERSION")" ||
    fail "Invalid configured minimum macOS version: $EXPECTED_MINIMUM_SYSTEM_VERSION"
  for architecture in arm64 x86_64; do
    actual_version="$(minimum_macos_for_architecture "$executable_path" "$architecture")"
    [[ -n "$actual_version" ]] ||
      fail "Cannot determine the $architecture deployment target for $executable_path"
    actual_number="$(version_number "$actual_version")" ||
      fail "Invalid $architecture deployment target '$actual_version' in $executable_path"
    if (( actual_number > expected_number )); then
      fail "$executable_path requires macOS $actual_version for $architecture, above the declared $EXPECTED_MINIMUM_SYSTEM_VERSION minimum"
    fi
  done
}

guard_absent() {
  local path="$1"
  local description="$2"

  [[ ! -e "$path" && ! -L "$path" ]] ||
    fail "Refusing to overwrite stale $description: $path. Move or remove it explicitly, then retry."
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "App Store packages must be built on macOS." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1 && command -v brew >/dev/null 2>&1; then
  RUSTUP_BIN="$(brew --prefix rustup 2>/dev/null)/bin"
  if [[ -d "$RUSTUP_BIN" ]]; then
    export PATH="$RUSTUP_BIN:$PATH"
  fi
fi

for command_name in npm cargo rustup xcrun codesign security plutil lipo otool xattr jq shasum stat find awk grep sort wc tr pkgutil file; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    exit 1
  fi
done

: "${APPLE_TEAM_ID:?Set APPLE_TEAM_ID to your 10-character Apple Developer Team ID.}"
: "${APPLE_SIGNING_IDENTITY:?Set APPLE_SIGNING_IDENTITY to your Apple Distribution certificate name.}"
: "${APPLE_INSTALLER_SIGNING_IDENTITY:?Set APPLE_INSTALLER_SIGNING_IDENTITY to your Mac Installer Distribution certificate name.}"
: "${APPLE_PROVISIONING_PROFILE:?Set APPLE_PROVISIONING_PROFILE to the Mac App Store Connect provisioning profile path.}"
: "${APP_BUILD_NUMBER:?Set APP_BUILD_NUMBER explicitly (5 is the next available App Store build).}"

if [[ ! "$APPLE_TEAM_ID" =~ ^[A-Z0-9]{10}$ ]]; then
  echo "APPLE_TEAM_ID must be the 10-character Apple Developer Team ID." >&2
  exit 1
fi

if [[ ! "$APP_BUILD_NUMBER" =~ ^[1-9][0-9]{0,8}$ ]]; then
  echo "APP_BUILD_NUMBER must be a positive integer of at most nine digits." >&2
  exit 1
fi

if (( APP_BUILD_NUMBER < MINIMUM_RELEASE_BUILD_NUMBER )); then
  fail "APP_BUILD_NUMBER must be at least $MINIMUM_RELEASE_BUILD_NUMBER; builds 1 through 4 have already been used."
fi

require_regular_file "$APPLE_PROVISIONING_PROFILE" "App Store provisioning profile"
require_regular_file "$HELPER_INPUT" "Universal llama-server sidecar"
[[ -x "$HELPER_INPUT" ]] ||
  fail "Universal llama-server sidecar is not executable: $HELPER_INPUT. Run ./scripts/build-llama-sidecar.sh."
verify_exact_model "$MODEL_INPUT"
require_regular_file "$NOTICE_INPUT" "Bundled model notice"
require_regular_file "$QWEN_LICENSE_INPUT" "Bundled Qwen license"
require_regular_file "$LLAMA_CPP_LICENSE_INPUT" "Bundled llama.cpp license"
require_regular_file "$CPP_HTTPLIB_LICENSE_INPUT" "Bundled cpp-httplib license"
require_regular_file "$NLOHMANN_JSON_LICENSE_INPUT" "Bundled nlohmann/json license"
require_regular_file "$CLI_INPUT" "Bundled ph7 launcher"
[[ -x "$CLI_INPUT" ]] || fail "Bundled ph7 launcher is not executable: $CLI_INPUT"
require_regular_file "$TAURI_DIR/Cargo.lock" "Rust dependency lockfile"
require_regular_file "$ROOT_DIR/package-lock.json" "Frontend dependency lockfile"

reject_release_xattrs "$APPLE_PROVISIONING_PROFILE"
reject_release_xattrs "$HELPER_INPUT"
reject_release_xattrs "$MODEL_INPUT"
for release_resource in \
  "$NOTICE_INPUT" \
  "$QWEN_LICENSE_INPUT" \
  "$LLAMA_CPP_LICENSE_INPUT" \
  "$CPP_HTTPLIB_LICENSE_INPUT" \
  "$NLOHMANN_JSON_LICENSE_INPUT" \
  "$CLI_INPUT"; do
  reject_release_xattrs "$release_resource"
done

# Never let Tauri or productbuild merge a previous release into a new one. This
# deliberately fails instead of deleting release output behind the operator's
# back. Temporary inputs below are guarded separately and cleaned only when
# this invocation created them.
guard_absent "$APP_PATH" "application bundle"
guard_absent "$PKG_PATH" "installer package"
guard_absent "$ENTITLEMENTS" "generated app entitlements"
guard_absent "$EMBEDDED_PROFILE" "generated embedded provisioning profile"
guard_absent "$GENERATED_CONFIG" "generated App Store configuration"

verify_exact_universal_binary "$HELPER_INPUT"
verify_deployment_target "$HELPER_INPUT"

SENSITIVE_RELEASE_INPUT="$({
  find "$ROOT_DIR" \
    -path "$ROOT_DIR/.git" -prune -o \
    -path "$ROOT_DIR/node_modules" -prune -o \
    -path "$TAURI_DIR/target" -prune -o \
    -type f \( -name 'AuthKey_*.p8' -o -name '*.p12' -o -name '*.key' \) -print -quit
} 2>/dev/null)"
if [[ -n "$SENSITIVE_RELEASE_INPUT" ]]; then
  fail "Refusing to build while a private key-like file is inside the repository: $SENSITIVE_RELEASE_INPUT"
fi

APP_VERSION="$(jq -er '.version | select(type == "string" and test("^[0-9]+(\\.[0-9]+){1,2}$"))' "$TAURI_DIR/tauri.conf.json")" ||
  fail "src-tauri/tauri.conf.json must contain a numeric macOS app version."
FRONTEND_VERSION="$(jq -er '.version' "$ROOT_DIR/package.json")" ||
  fail "Cannot read the frontend package version."
LOCKFILE_VERSION="$(jq -er '.packages[""].version' "$ROOT_DIR/package-lock.json")" ||
  fail "Cannot read the root package-lock version."
CARGO_VERSION="$(
  cargo metadata --locked --offline --no-deps --format-version 1 --manifest-path "$TAURI_DIR/Cargo.toml" |
    jq -er '.packages[] | select(.name == "ph7-console") | .version'
)" || fail "Cannot read the locked Rust package version without network access."

if [[ "$FRONTEND_VERSION" != "$APP_VERSION" || "$LOCKFILE_VERSION" != "$APP_VERSION" || "$CARGO_VERSION" != "$APP_VERSION" ]]; then
  fail "Release versions disagree: Tauri=$APP_VERSION package.json=$FRONTEND_VERSION package-lock.json=$LOCKFILE_VERSION Cargo=$CARGO_VERSION"
fi

jq -e \
  --arg identifier "$APP_IDENTIFIER" \
  --arg minimum "$EXPECTED_MINIMUM_SYSTEM_VERSION" \
  '.identifier == $identifier and
   .bundle.macOS.minimumSystemVersion == $minimum and
   .bundle.externalBin == ["binaries/llama-server"]' \
  "$TAURI_DIR/tauri.conf.json" >/dev/null ||
  fail "Base Tauri identifier, minimum macOS version, or llama-server external binary configuration is unexpected."

jq -e \
  --arg minimum "$EXPECTED_MINIMUM_SYSTEM_VERSION" \
  '.bundle.macOS.minimumSystemVersion == $minimum and
   .bundle.macOS.bundleVersion == "1" and
   (.bundle.resources | length) == 8 and
   .bundle.resources["PrivacyInfo.xcprivacy"] == "PrivacyInfo.xcprivacy" and
   .bundle.resources["resources/models/terminal-assistant.gguf"] == "resources/models/terminal-assistant.gguf" and
   .bundle.resources["resources/models/NOTICE.md"] == "resources/models/NOTICE.md" and
   .bundle.resources["resources/models/LICENSE-QWEN"] == "resources/models/LICENSE-QWEN" and
   .bundle.resources["resources/models/LICENSE-LLAMA-CPP"] == "resources/models/LICENSE-LLAMA-CPP" and
   .bundle.resources["resources/models/LICENSE-CPP-HTTPLIB"] == "resources/models/LICENSE-CPP-HTTPLIB" and
   .bundle.resources["resources/models/LICENSE-NLOHMANN-JSON"] == "resources/models/LICENSE-NLOHMANN-JSON" and
   .bundle.resources["../scripts/ph7"] == "bin/ph7"' \
  "$TAURI_DIR/tauri.appstore.conf.json" >/dev/null ||
  fail "App Store configuration must retain the checked version, deployment target, and deterministic resource map."

verify_system_linkage() {
  local executable_path="$1"
  local architecture
  local dependency_path
  local invalid_dependency=0

  for architecture in arm64 x86_64; do
    while IFS= read -r dependency_path; do
      [[ -z "$dependency_path" ]] && continue
      case "$dependency_path" in
        /System/Library/* | /usr/lib/*)
          ;;
        *)
          echo "Unexpected $architecture dependency in $executable_path: $dependency_path" >&2
          invalid_dependency=1
          ;;
      esac
    done < <(otool -arch "$architecture" -L "$executable_path" | awk 'NR > 1 { print $1 }')
  done

  if [[ "$invalid_dependency" -ne 0 ]]; then
    return 1
  fi
}

verify_system_linkage "$HELPER_INPUT"

if ! security find-identity -v -p codesigning | grep -F "$APPLE_SIGNING_IDENTITY" >/dev/null; then
  echo "Apple Distribution signing identity is not installed in the keychain." >&2
  exit 1
fi

if ! security find-certificate -a -c "$APPLE_INSTALLER_SIGNING_IDENTITY" >/dev/null; then
  echo "Mac Installer Distribution certificate is not installed in the keychain." >&2
  exit 1
fi

PROFILE_PLIST="$(mktemp -t ph7console-profile)"
SIGNED_ENTITLEMENTS="$(mktemp -t ph7console-entitlements)"
SIGNED_HELPER_ENTITLEMENTS="$(mktemp -t ph7console-helper-entitlements)"
EXPANDED_PACKAGE_PARENT="$(mktemp -d -t ph7console-package)"
EXPANDED_PACKAGE="$EXPANDED_PACKAGE_PARENT/expanded"

cleanup() {
  rm -f "$ENTITLEMENTS" "$EMBEDDED_PROFILE" "$GENERATED_CONFIG" "$PROFILE_PLIST" "$SIGNED_ENTITLEMENTS" "$SIGNED_HELPER_ENTITLEMENTS"
  rm -rf "$EXPANDED_PACKAGE_PARENT"
}
trap cleanup EXIT

security cms -D -i "$APPLE_PROVISIONING_PROFILE" > "$PROFILE_PLIST"
PROFILE_APP_ID="$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:com.apple.application-identifier' "$PROFILE_PLIST" 2>/dev/null || true)"
if [[ -z "$PROFILE_APP_ID" ]]; then
  PROFILE_APP_ID="$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:application-identifier' "$PROFILE_PLIST" 2>/dev/null || true)"
fi
PROFILE_TEAM_ID="$(/usr/libexec/PlistBuddy -c 'Print :TeamIdentifier:0' "$PROFILE_PLIST" 2>/dev/null || true)"
EXPECTED_APP_ID="$APPLE_TEAM_ID.$APP_IDENTIFIER"

if [[ "$PROFILE_APP_ID" != "$EXPECTED_APP_ID" ]]; then
  echo "Provisioning profile is for '$PROFILE_APP_ID'; expected '$EXPECTED_APP_ID'." >&2
  exit 1
fi

if [[ "$PROFILE_TEAM_ID" != "$APPLE_TEAM_ID" ]]; then
  fail "Provisioning profile Team '$PROFILE_TEAM_ID' does not match '$APPLE_TEAM_ID'."
fi

# App Sandbox entitlements are unrestricted on macOS and therefore do not
# need to appear in a provisioning profile (Apple TN3125). The signed app and
# helper entitlement sets are verified strictly below, including sandbox=true.

cp "$TAURI_DIR/Entitlements.plist.template" "$ENTITLEMENTS"
sed -i '' "s/__APPLE_TEAM_ID__/$APPLE_TEAM_ID/g" "$ENTITLEMENTS"
cp "$APPLE_PROVISIONING_PROFILE" "$EMBEDDED_PROFILE"
cp "$TAURI_DIR/tauri.appstore.conf.json" "$GENERATED_CONFIG"
sed -i '' -E "s/\"bundleVersion\": \"[0-9]+\"/\"bundleVersion\": \"$APP_BUILD_NUMBER\"/" "$GENERATED_CONFIG"

plutil -lint "$TAURI_DIR/Info.plist" "$ENTITLEMENTS" "$HELPER_ENTITLEMENTS"
jq empty "$GENERATED_CONFIG"
jq -e --arg build "$APP_BUILD_NUMBER" '.bundle.macOS.bundleVersion == $build' "$GENERATED_CONFIG" >/dev/null ||
  fail "Generated App Store configuration does not contain build $APP_BUILD_NUMBER."

EXPECTED_APP_ENTITLEMENT_KEYS=$'com.apple.application-identifier\ncom.apple.developer.team-identifier\ncom.apple.security.app-sandbox\ncom.apple.security.device.microphone\ncom.apple.security.files.user-selected.executable\ncom.apple.security.files.user-selected.read-write\ncom.apple.security.network.client\ncom.apple.security.network.server'
GENERATED_APP_ENTITLEMENT_KEYS="$(
  plutil -convert json -o - "$ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"
if [[ "$GENERATED_APP_ENTITLEMENT_KEYS" != "$EXPECTED_APP_ENTITLEMENT_KEYS" ]]; then
  fail "Generated application entitlements contain a missing or unexpected key."
fi

EXPECTED_HELPER_KEYS=$'com.apple.security.app-sandbox\ncom.apple.security.inherit'
GENERATED_HELPER_KEYS="$(
  plutil -convert json -o - "$HELPER_ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"
if [[ "$GENERATED_HELPER_KEYS" != "$EXPECTED_HELPER_KEYS" ]]; then
  fail "Helper entitlements contain a missing or unexpected key."
fi

for rust_target in aarch64-apple-darwin x86_64-apple-darwin; do
  if ! rustup target list --installed | grep -Fx "$rust_target" >/dev/null; then
    fail "Rust target '$rust_target' is not installed. Install release toolchains before running this offline build."
  fi
done

CARGO_LOCK_SHA256="$(sha256_file "$TAURI_DIR/Cargo.lock")"
PACKAGE_LOCK_SHA256="$(sha256_file "$ROOT_DIR/package-lock.json")"

cd "$ROOT_DIR"
export CARGO_NET_OFFLINE=true
export CARGO_INCREMENTAL=0
npm run tauri -- build \
  --ci \
  --bundles app \
  --target universal-apple-darwin \
  --no-sign \
  --config src-tauri/tauri.appstore.generated.conf.json

[[ "$(sha256_file "$TAURI_DIR/Cargo.lock")" == "$CARGO_LOCK_SHA256" ]] ||
  fail "Cargo.lock changed during the release build. Refusing a non-reproducible artifact."
[[ "$(sha256_file "$ROOT_DIR/package-lock.json")" == "$PACKAGE_LOCK_SHA256" ]] ||
  fail "package-lock.json changed during the release build. Refusing a non-reproducible artifact."

APP_EXECUTABLE="$APP_PATH/Contents/MacOS/ph7-console"
HELPER_EXECUTABLE="$APP_PATH/Contents/MacOS/llama-server"
BUNDLED_INFO_PLIST="$APP_PATH/Contents/Info.plist"
BUNDLED_PRIVACY_MANIFEST="$APP_PATH/Contents/Resources/PrivacyInfo.xcprivacy"
BUNDLED_MODEL="$APP_PATH/Contents/Resources/$MODEL_RESOURCE_RELATIVE"
BUNDLED_NOTICE="$APP_PATH/Contents/Resources/resources/models/NOTICE.md"
BUNDLED_QWEN_LICENSE="$APP_PATH/Contents/Resources/resources/models/LICENSE-QWEN"
BUNDLED_LLAMA_CPP_LICENSE="$APP_PATH/Contents/Resources/resources/models/LICENSE-LLAMA-CPP"
BUNDLED_CPP_HTTPLIB_LICENSE="$APP_PATH/Contents/Resources/resources/models/LICENSE-CPP-HTTPLIB"
BUNDLED_NLOHMANN_JSON_LICENSE="$APP_PATH/Contents/Resources/resources/models/LICENSE-NLOHMANN-JSON"
BUNDLED_CLI="$APP_PATH/Contents/Resources/bin/ph7"

[[ -d "$APP_PATH" && ! -L "$APP_PATH" ]] || fail "Tauri did not produce the expected application bundle: $APP_PATH"
require_regular_file "$APP_EXECUTABLE" "Application executable"
[[ -x "$APP_EXECUTABLE" ]] || fail "Application executable is not executable: $APP_EXECUTABLE"

require_regular_file "$HELPER_EXECUTABLE" "Bundled llama-server helper"
[[ -x "$HELPER_EXECUTABLE" ]] || fail "Bundled llama-server helper is not executable: $HELPER_EXECUTABLE"

verify_exact_universal_binary "$APP_EXECUTABLE"
verify_exact_universal_binary "$HELPER_EXECUTABLE"
verify_deployment_target "$APP_EXECUTABLE"
verify_deployment_target "$HELPER_EXECUTABLE"
verify_system_linkage "$APP_EXECUTABLE"
verify_system_linkage "$HELPER_EXECUTABLE"

if [[ "$(sha256_file "$HELPER_EXECUTABLE")" != "$(sha256_file "$HELPER_INPUT")" ]]; then
  fail "Bundled llama-server does not byte-match the verified universal build input."
fi

verify_exact_model "$BUNDLED_MODEL"
if [[ "$(sha256_file "$BUNDLED_MODEL")" != "$(sha256_file "$MODEL_INPUT")" ]]; then
  fail "Bundled Qwen GGUF does not byte-match the pinned release input."
fi

verify_resource_copy() {
  local source_path="$1"
  local bundled_path="$2"
  local description="$3"

  require_regular_file "$bundled_path" "$description"
  if [[ "$(sha256_file "$source_path")" != "$(sha256_file "$bundled_path")" ]]; then
    fail "$description changed while it was copied into the app: $bundled_path"
  fi
}

verify_resource_copy "$NOTICE_INPUT" "$BUNDLED_NOTICE" "Bundled model notice"
verify_resource_copy "$QWEN_LICENSE_INPUT" "$BUNDLED_QWEN_LICENSE" "Bundled Qwen license"
verify_resource_copy "$LLAMA_CPP_LICENSE_INPUT" "$BUNDLED_LLAMA_CPP_LICENSE" "Bundled llama.cpp license"
verify_resource_copy "$CPP_HTTPLIB_LICENSE_INPUT" "$BUNDLED_CPP_HTTPLIB_LICENSE" "Bundled cpp-httplib license"
verify_resource_copy "$NLOHMANN_JSON_LICENSE_INPUT" "$BUNDLED_NLOHMANN_JSON_LICENSE" "Bundled nlohmann/json license"
verify_resource_copy "$CLI_INPUT" "$BUNDLED_CLI" "Bundled ph7 launcher"
[[ -x "$BUNDLED_CLI" ]] || fail "Bundled ph7 launcher lost its executable mode: $BUNDLED_CLI"

GGUF_COUNT="$(find "$APP_PATH/Contents" -type f -name '*.gguf' | wc -l | tr -d '[:space:]')"
[[ "$GGUF_COUNT" == "1" ]] || fail "App bundle must contain exactly one GGUF model; found $GGUF_COUNT."
HELPER_COUNT="$(find "$APP_PATH/Contents" -type f -name 'llama-server*' | wc -l | tr -d '[:space:]')"
[[ "$HELPER_COUNT" == "1" ]] || fail "App bundle must contain exactly one llama-server helper; found $HELPER_COUNT."

CONTAMINATED_PATH="$(
  find "$APP_PATH" \( \
    -name '.DS_Store' -o \
    -name '__MACOSX' -o \
    -name '.git' -o \
    -name '*.part' -o \
    -name '*.p8' -o \
    -name '*.p12' -o \
    -name '*.key' \
  \) -print -quit
)"
[[ -z "$CONTAMINATED_PATH" ]] || fail "App bundle contains a forbidden release artifact: $CONTAMINATED_PATH"

RESOURCE_SYMLINK="$(find "$APP_PATH/Contents/Resources" -type l -print -quit)"
[[ -z "$RESOURCE_SYMLINK" ]] || fail "App resources must not contain symbolic links: $RESOURCE_SYMLINK"

while IFS= read -r -d '' bundled_file; do
  if file -b "$bundled_file" | grep -F 'Mach-O' >/dev/null; then
    case "$bundled_file" in
      "$APP_EXECUTABLE" | "$HELPER_EXECUTABLE") ;;
      *) fail "Unexpected nested Mach-O code in App Store bundle: $bundled_file" ;;
    esac
  fi
done < <(find "$APP_PATH/Contents" -type f -print0)

while IFS= read -r -d '' release_path; do
  reject_release_xattrs "$release_path"
done < <(find "$APP_PATH" -print0)

require_regular_file "$BUNDLED_INFO_PLIST" "Bundled Info.plist"
plutil -lint "$BUNDLED_INFO_PLIST" "$BUNDLED_PRIVACY_MANIFEST"

BUNDLED_IDENTIFIER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$BUNDLED_INFO_PLIST" 2>/dev/null || true)"
BUNDLED_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$BUNDLED_INFO_PLIST" 2>/dev/null || true)"
BUNDLED_BUILD="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$BUNDLED_INFO_PLIST" 2>/dev/null || true)"
BUNDLED_EXECUTABLE_NAME="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$BUNDLED_INFO_PLIST" 2>/dev/null || true)"
BUNDLED_MINIMUM_SYSTEM_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$BUNDLED_INFO_PLIST" 2>/dev/null || true)"

[[ "$BUNDLED_IDENTIFIER" == "$APP_IDENTIFIER" ]] ||
  fail "Bundled identifier '$BUNDLED_IDENTIFIER' does not match '$APP_IDENTIFIER'."
[[ "$BUNDLED_VERSION" == "$APP_VERSION" ]] ||
  fail "Bundled version '$BUNDLED_VERSION' does not match '$APP_VERSION'."
[[ "$BUNDLED_BUILD" == "$APP_BUILD_NUMBER" ]] ||
  fail "Bundled build '$BUNDLED_BUILD' does not match '$APP_BUILD_NUMBER'."
[[ "$BUNDLED_EXECUTABLE_NAME" == "ph7-console" ]] ||
  fail "Bundled executable name '$BUNDLED_EXECUTABLE_NAME' is unexpected."
[[ "$BUNDLED_MINIMUM_SYSTEM_VERSION" == "$EXPECTED_MINIMUM_SYSTEM_VERSION" ]] ||
  fail "Bundled minimum macOS version '$BUNDLED_MINIMUM_SYSTEM_VERSION' does not match '$EXPECTED_MINIMUM_SYSTEM_VERSION'."

plutil -convert json -o - "$BUNDLED_INFO_PLIST" | jq -e '
  .ITSAppUsesNonExemptEncryption == false and
  (.NSMicrophoneUsageDescription | type == "string" and length > 40) and
  (.NSSpeechRecognitionUsageDescription | type == "string" and length > 40) and
  any(.CFBundleURLTypes[]?; any(.CFBundleURLSchemes[]?; . == "ph7console"))
' >/dev/null || fail "Bundled Info.plist is missing a privacy usage description, the encryption declaration, or the ph7console URL scheme."

plutil -convert json -o - "$BUNDLED_PRIVACY_MANIFEST" | jq -e '
  .NSPrivacyTracking == false and
  (.NSPrivacyTrackingDomains | type == "array" and length == 0) and
  (.NSPrivacyCollectedDataTypes | type == "array" and length == 0) and
  (.NSPrivacyAccessedAPITypes | type == "array" and length == 0)
' >/dev/null || fail "Bundled privacy manifest does not match the no-tracking/no-collection release declaration."

# Tauri applies one entitlement file to every executable it signs. The App
# Store helper must instead inherit the parent sandbox with only the two
# helper entitlements, so sign nested code explicitly before sealing the app.
codesign --force \
  --sign "$APPLE_SIGNING_IDENTITY" \
  --identifier "$HELPER_IDENTIFIER" \
  --options runtime \
  --timestamp="$APPLE_TIMESTAMP_URL" \
  --generate-entitlement-der \
  --entitlements "$HELPER_ENTITLEMENTS" \
  "$HELPER_EXECUTABLE"
codesign --force \
  --sign "$APPLE_SIGNING_IDENTITY" \
  --options runtime \
  --timestamp="$APPLE_TIMESTAMP_URL" \
  --generate-entitlement-der \
  --entitlements "$ENTITLEMENTS" \
  "$APP_PATH"

codesign --verify --strict --verbose=2 "$HELPER_EXECUTABLE"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

require_regular_file "$APP_PATH/Contents/embedded.provisionprofile" "Embedded provisioning profile"
if [[ "$(sha256_file "$APP_PATH/Contents/embedded.provisionprofile")" != "$(sha256_file "$APPLE_PROVISIONING_PROFILE")" ]]; then
  fail "Embedded provisioning profile does not byte-match the selected release profile."
fi
verify_exact_model "$BUNDLED_MODEL"
verify_exact_universal_binary "$APP_EXECUTABLE"
verify_exact_universal_binary "$HELPER_EXECUTABLE"

codesign -d --xml --entitlements "$SIGNED_ENTITLEMENTS" "$APP_EXECUTABLE" 2>/dev/null
codesign -d --xml --entitlements "$SIGNED_HELPER_ENTITLEMENTS" "$HELPER_EXECUTABLE" 2>/dev/null
plutil -lint "$SIGNED_ENTITLEMENTS" "$SIGNED_HELPER_ENTITLEMENTS"

entitlement_value() {
  local key="$1"
  /usr/libexec/PlistBuddy -c "Print :$key" "$SIGNED_ENTITLEMENTS" 2>/dev/null || true
}

SIGNED_SANDBOX="$(entitlement_value 'com.apple.security.app-sandbox')"
SIGNED_APP_ID="$(entitlement_value 'com.apple.application-identifier')"
SIGNED_TEAM_ID="$(entitlement_value 'com.apple.developer.team-identifier')"
SIGNED_APP_KEYS="$(
  plutil -convert json -o - "$SIGNED_ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"

if [[ "$SIGNED_APP_KEYS" != "$EXPECTED_APP_ENTITLEMENT_KEYS" ]]; then
  fail "The signed application contains a missing or unexpected entitlement."
fi

if [[ "$SIGNED_SANDBOX" != "true" ]]; then
  echo "The signed app does not contain the App Sandbox entitlement." >&2
  exit 1
fi

for required_app_entitlement in \
  com.apple.security.device.microphone \
  com.apple.security.files.user-selected.executable \
  com.apple.security.files.user-selected.read-write \
  com.apple.security.network.client \
  com.apple.security.network.server; do
  if [[ "$(entitlement_value "$required_app_entitlement")" != "true" ]]; then
    fail "The signed app is missing required entitlement '$required_app_entitlement'."
  fi
done

if [[ "$SIGNED_APP_ID" != "$EXPECTED_APP_ID" || "$SIGNED_TEAM_ID" != "$APPLE_TEAM_ID" ]]; then
  echo "The signed app entitlements do not match the selected team and bundle ID." >&2
  exit 1
fi

helper_entitlement_value() {
  local key="$1"
  /usr/libexec/PlistBuddy -c "Print :$key" "$SIGNED_HELPER_ENTITLEMENTS" 2>/dev/null || true
}

EXPECTED_HELPER_KEYS=$'com.apple.security.app-sandbox\ncom.apple.security.inherit'
SIGNED_HELPER_KEYS="$(
  plutil -convert json -o - "$SIGNED_HELPER_ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"

if [[ "$SIGNED_HELPER_KEYS" != "$EXPECTED_HELPER_KEYS" ]]; then
  echo "The signed llama-server helper contains an unexpected entitlement set." >&2
  exit 1
fi

if [[ "$(helper_entitlement_value 'com.apple.security.app-sandbox')" != "true" || \
      "$(helper_entitlement_value 'com.apple.security.inherit')" != "true" ]]; then
  echo "The signed llama-server helper does not inherit the App Sandbox." >&2
  exit 1
fi

APP_SIGNATURE_DETAILS="$(codesign -dv --verbose=4 "$APP_EXECUTABLE" 2>&1)"
HELPER_SIGNATURE_DETAILS="$(codesign -dv --verbose=4 "$HELPER_EXECUTABLE" 2>&1)"

if ! grep -Fx "Identifier=$APP_IDENTIFIER" <<<"$APP_SIGNATURE_DETAILS" >/dev/null; then
  fail "The signed application has an unexpected stable code identifier."
fi
if ! grep -Fx "TeamIdentifier=$APPLE_TEAM_ID" <<<"$APP_SIGNATURE_DETAILS" >/dev/null; then
  fail "The signed application does not match the selected Apple Team."
fi
if ! grep -Fx "Identifier=$HELPER_IDENTIFIER" <<<"$HELPER_SIGNATURE_DETAILS" >/dev/null; then
  fail "The signed llama-server identifier is not '$HELPER_IDENTIFIER'."
fi
if ! grep -Fx "TeamIdentifier=$APPLE_TEAM_ID" <<<"$HELPER_SIGNATURE_DETAILS" >/dev/null; then
  fail "The signed llama-server helper does not match the selected Apple Team."
fi
if grep -F 'Signature=adhoc' <<<"$APP_SIGNATURE_DETAILS" >/dev/null ||
   grep -F 'Signature=adhoc' <<<"$HELPER_SIGNATURE_DETAILS" >/dev/null; then
  fail "A release executable still has an ad-hoc signature."
fi

# Verify again after the outer resource seal is created. This catches any
# accidental post-sign resource mutation before productbuild runs.
codesign --verify --strict --verbose=2 "$HELPER_EXECUTABLE"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"
verify_exact_model "$BUNDLED_MODEL"

mkdir -p "$OUTPUT_DIR"
xcrun productbuild \
  --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
  --component "$APP_PATH" /Applications \
  "$PKG_PATH"

require_regular_file "$PKG_PATH" "App Store installer package"
pkgutil --check-signature "$PKG_PATH"

# Validate the exact payload that will be uploaded, not only the source bundle.
# This catches packaging or post-sign mutations that can otherwise leave the
# App Store with an app whose buttons cannot reach their native commands.
pkgutil --expand-full "$PKG_PATH" "$EXPANDED_PACKAGE"
PACKAGED_APP_COUNT="$(find "$EXPANDED_PACKAGE" -type d -name 'pH7Console.app' -prune -print | wc -l | tr -d '[:space:]')"
[[ "$PACKAGED_APP_COUNT" == "1" ]] ||
  fail "Installer must contain exactly one pH7Console.app; found $PACKAGED_APP_COUNT."
PACKAGED_APP="$(find "$EXPANDED_PACKAGE" -type d -name 'pH7Console.app' -prune -print -quit)"
PACKAGED_APP_EXECUTABLE="$PACKAGED_APP/Contents/MacOS/ph7-console"
PACKAGED_HELPER_EXECUTABLE="$PACKAGED_APP/Contents/MacOS/llama-server"
PACKAGED_MODEL="$PACKAGED_APP/Contents/Resources/$MODEL_RESOURCE_RELATIVE"
PACKAGED_INFO_PLIST="$PACKAGED_APP/Contents/Info.plist"

codesign --verify --strict --verbose=2 "$PACKAGED_HELPER_EXECUTABLE"
codesign --verify --deep --strict --verbose=2 "$PACKAGED_APP"
codesign -d --xml --entitlements "$SIGNED_ENTITLEMENTS" "$PACKAGED_APP_EXECUTABLE" 2>/dev/null
codesign -d --xml --entitlements "$SIGNED_HELPER_ENTITLEMENTS" "$PACKAGED_HELPER_EXECUTABLE" 2>/dev/null
plutil -lint "$SIGNED_ENTITLEMENTS" "$SIGNED_HELPER_ENTITLEMENTS" "$PACKAGED_INFO_PLIST"

PACKAGED_APP_KEYS="$(
  plutil -convert json -o - "$SIGNED_ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"
PACKAGED_HELPER_KEYS="$(
  plutil -convert json -o - "$SIGNED_HELPER_ENTITLEMENTS" | jq -r 'keys[]' | LC_ALL=C sort
)"
[[ "$PACKAGED_APP_KEYS" == "$EXPECTED_APP_ENTITLEMENT_KEYS" ]] ||
  fail "Packaged application contains a missing, invalid, or unexpected entitlement."
[[ "$PACKAGED_HELPER_KEYS" == "$EXPECTED_HELPER_KEYS" ]] ||
  fail "Packaged llama-server helper contains a missing, invalid, or unexpected entitlement."
[[ "$(entitlement_value 'com.apple.security.app-sandbox')" == "true" ]] ||
  fail "Packaged application lost its App Sandbox entitlement."
[[ "$(entitlement_value 'com.apple.security.network.server')" == "true" ]] ||
  fail "Packaged application lost the loopback server entitlement required by local inference."
[[ "$(helper_entitlement_value 'com.apple.security.app-sandbox')" == "true" && \
   "$(helper_entitlement_value 'com.apple.security.inherit')" == "true" ]] ||
  fail "Packaged llama-server helper no longer inherits the parent App Sandbox."

[[ "$(sha256_file "$PACKAGED_APP_EXECUTABLE")" == "$(sha256_file "$APP_EXECUTABLE")" ]] ||
  fail "Packaged application executable differs from the verified signed source."
[[ "$(sha256_file "$PACKAGED_HELPER_EXECUTABLE")" == "$(sha256_file "$HELPER_EXECUTABLE")" ]] ||
  fail "Packaged llama-server helper differs from the verified signed source."
verify_exact_model "$PACKAGED_MODEL"
[[ "$(sha256_file "$PACKAGED_MODEL")" == "$(sha256_file "$BUNDLED_MODEL")" ]] ||
  fail "Packaged model differs from the verified bundled model."
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$PACKAGED_INFO_PLIST")" == "$APP_BUILD_NUMBER" ]] ||
  fail "Packaged application does not contain build $APP_BUILD_NUMBER."

echo "App Store package ready: $PKG_PATH"
