#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="${PH7_LLAMA_SOURCE_DIR:-$ROOT_DIR/src-tauri/target/llama.cpp-b9637}"
BUILD_ROOT="${PH7_LLAMA_BUILD_DIR:-$ROOT_DIR/src-tauri/target/llama-sidecar}"
OUTPUT_DIR="$ROOT_DIR/src-tauri/binaries"
PINNED_COMMIT="aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3"

for command_name in git cmake lipo; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required build command: $command_name" >&2
    exit 1
  fi
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The universal Metal sidecar must be built on macOS." >&2
  exit 1
fi

if [[ ! -d "$SOURCE_DIR/.git" ]]; then
  git clone --filter=blob:none --no-checkout https://github.com/ggml-org/llama.cpp.git "$SOURCE_DIR"
fi

git -C "$SOURCE_DIR" fetch --depth 1 origin "$PINNED_COMMIT"
git -C "$SOURCE_DIR" checkout --detach "$PINNED_COMMIT"

if [[ "$(git -C "$SOURCE_DIR" rev-parse HEAD)" != "$PINNED_COMMIT" ]]; then
  echo "Refusing to build an unpinned llama.cpp revision." >&2
  exit 1
fi

build_slice() {
  local architecture="$1"
  local build_dir="$BUILD_ROOT-$architecture"
  local metal="ON"

  # Match llama.cpp's official macOS release policy. Its current Accelerate
  # interface requires macOS 13.3, and upstream disables Metal for Intel
  # release slices because it is unreliable on headless Intel builders.
  if [[ "$architecture" == "x86_64" ]]; then
    metal="OFF"
  fi

  cmake -S "$SOURCE_DIR" -B "$build_dir" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_OSX_ARCHITECTURES="$architecture" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=13.3 \
    -DBUILD_SHARED_LIBS=OFF \
    -DGGML_NATIVE=OFF \
    -DGGML_ACCELERATE=ON \
    -DGGML_METAL="$metal" \
    -DGGML_METAL_EMBED_LIBRARY=ON \
    -DLLAMA_BUILD_TESTS=OFF \
    -DLLAMA_BUILD_EXAMPLES=ON \
    -DLLAMA_BUILD_SERVER=ON \
    -DLLAMA_BUILD_UI=OFF \
    -DLLAMA_USE_PREBUILT_UI=OFF \
    -DLLAMA_CURL=OFF \
    -DLLAMA_OPENSSL=OFF
  # A prior developer build may have left UI assets in CMake's output tree.
  # Clear only that generated directory so the no-UI release cannot embed it.
  cmake -E remove_directory "$build_dir/tools/ui/dist"
  cmake --build "$build_dir" --config Release --target llama-server --parallel
  if [[ ! -x "$build_dir/bin/llama-server" ]]; then
    echo "llama-server $architecture slice was not produced." >&2
    exit 1
  fi
}

build_slice arm64
build_slice x86_64

mkdir -p "$BUILD_ROOT-universal/bin"
SIDE_CAR="$BUILD_ROOT-universal/bin/llama-server"
lipo -create \
  "$BUILD_ROOT-arm64/bin/llama-server" \
  "$BUILD_ROOT-x86_64/bin/llama-server" \
  -output "$SIDE_CAR"
lipo "$SIDE_CAR" -verify_arch arm64 x86_64

mkdir -p "$OUTPUT_DIR"
install -m 0755 "$SIDE_CAR" "$OUTPUT_DIR/llama-server-aarch64-apple-darwin"
install -m 0755 "$SIDE_CAR" "$OUTPUT_DIR/llama-server-x86_64-apple-darwin"
install -m 0755 "$SIDE_CAR" "$OUTPUT_DIR/llama-server-universal-apple-darwin"

echo "Pinned universal llama-server sidecar is ready in $OUTPUT_DIR"
