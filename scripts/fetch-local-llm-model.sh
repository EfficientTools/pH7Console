#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_DIR="$ROOT_DIR/src-tauri/resources/models"
MODEL_PATH="$MODEL_DIR/terminal-assistant.gguf"
PARTIAL_PATH="$MODEL_PATH.part"
MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/f86cb2c1fa58255f8052cc32aeede1b7482d4361/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf"
MODEL_SHA256="cc324af070c2ecbfd324a30884d2f951a7ff756aba85cb811a6ec436933bb046"
MODEL_SIZE="1117320768"

for command_name in curl shasum stat; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required download command: $command_name" >&2
    exit 1
  fi
done

verify_model() {
  local candidate="$1"
  [[ -f "$candidate" ]] || return 1
  [[ "$(stat -f %z "$candidate")" == "$MODEL_SIZE" ]] || return 1
  [[ "$(shasum -a 256 "$candidate" | awk '{print $1}')" == "$MODEL_SHA256" ]]
}

mkdir -p "$MODEL_DIR"
if verify_model "$MODEL_PATH"; then
  echo "Verified local model already present: $MODEL_PATH"
  exit 0
fi

curl -L --fail --retry 3 --continue-at - --output "$PARTIAL_PATH" "$MODEL_URL"
if ! verify_model "$PARTIAL_PATH"; then
  echo "Downloaded model failed its pinned size or SHA-256 check." >&2
  exit 1
fi

mv "$PARTIAL_PATH" "$MODEL_PATH"
chmod 0444 "$MODEL_PATH"
echo "Verified local model installed: $MODEL_PATH"
