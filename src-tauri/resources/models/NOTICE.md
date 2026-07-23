# Bundled local terminal model

pH7Console uses the official `Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF`
`q4_k_m` artifact for optional, fully local command generation.

- Upstream revision: `f86cb2c1fa58255f8052cc32aeede1b7482d4361`
- Expected size: `1,117,320,768` bytes
- SHA-256: `cc324af070c2ecbfd324a30884d2f951a7ff756aba85cb811a6ec436933bb046`
- License: Apache License 2.0
- Source: <https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF>

The model runs only through the bundled loopback llama.cpp helper. Prompts,
terminal output, command history, and generated tokens are not sent to Qwen,
Hugging Face, or any cloud inference provider.

## Bundled local inference runtime

pH7Console includes a statically linked `llama-server` built from
`ggml-org/llama.cpp` commit
`aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3` (release b9637).

- llama.cpp license: MIT (`LICENSE-LLAMA-CPP`)
- cpp-httplib license: MIT (`LICENSE-CPP-HTTPLIB`)
- nlohmann/json license: MIT (`LICENSE-NLOHMANN-JSON`)
- Source: <https://github.com/ggml-org/llama.cpp>

The helper listens only on an authenticated numeric loopback address and is
not a cloud service. These license files are bundled with every application
copy alongside this notice.
