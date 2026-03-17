# pH7Console: AI-Powered Terminal

![pH7Console Logo](src-tauri/icons/icon.png "ML-first terminal alternative - AI-driven terminal")

A privacy-first terminal built with Tauri that runs AI models locally — no telemetry, no cloud, no data leaving your machine.

## Usage

Type commands naturally — the AI translates them:

```
"show me all large files"  →  find . -type f -size +100M -exec ls -lh {} \;
"what's using the most CPU?"  →  top -o cpu
"check git status and stage changes"  →  git status && git add .
```

AI slash commands:
- `/explain <command>` — explain what any command does
- `/fix` — analyse the last error and suggest a fix
- `/optimize` — suggest a more efficient alternative

## Features

- **Natural Language Commands** — Type plain English; get shell commands
- **Smart Completions** — Context-aware Tab suggestions
- **Error Assistance** — AI automatically suggests fixes when commands fail
- **Local LLM Processing** — All inference runs on your machine; nothing leaves it
- **Pattern Learning** — Learns your workflows and adapts suggestions over time
- **Multi-Session** — `Cmd+T` new session, `Cmd+W` close, `Cmd+1–9` switch

## Requirements

- **Rust** 1.70+ — [rustup.rs](https://rustup.rs/)
- **Node.js** 18+ — [nodejs.org](https://nodejs.org/)
- **RAM** 4GB minimum (8GB recommended for AI models)
- **Storage** ~5GB free (for models and dependencies)

## Install & Run

```bash
git clone https://github.com/EfficientTools/pH7Console.git
cd pH7Console
chmod +x setup.sh && ./setup.sh
npm run tauri dev
```

## Build

```bash
# Production build
npm run tauri build

# Universal macOS binary (Intel + Apple Silicon)
npm run tauri build -- --target universal-apple-darwin
```

Build outputs land in `src-tauri/target/release/bundle/`.

## Development

```bash
npm run lint                   # TypeScript/React linting
npm run type-check             # TypeScript type checking
npm test                       # Frontend tests
cd src-tauri && cargo test     # Rust backend tests
cd src-tauri && cargo fmt      # Format Rust code
cd src-tauri && cargo clippy   # Lint Rust code
npm run test:e2e               # Integration tests
```

## Local AI Models

| Model | Size | RAM | Speed | Best for |
|-------|------|-----|-------|----------|
| Phi-3 Mini | 3.8 GB | 4–6 GB | 200–500 ms | Complex reasoning, code generation |
| Llama 3.2 1B | 1.2 GB | 2–3 GB | 100–200 ms | General commands, explanations |
| TinyLlama | 1.1 GB | 1.5–2 GB | 50–100 ms | Real-time completions |
| CodeQwen | 1.5 GB | 2–4 GB | 150–300 ms | Programming tasks, code analysis |

## Tech Stack

- **Frontend**: React 18 + TypeScript + Tailwind CSS
- **Backend**: Rust + Tauri 2.0
- **AI Runtime**: Candle (Rust-native ML framework)
- **Terminal**: xterm.js + cross-platform PTY

## Author

[![Pierre-Henry Soria](https://avatars0.githubusercontent.com/u/1325411?s=200)](https://ph7.me "Pierre-Henry Soria, Software Developer")

Made with ❤️ by **[Pierre-Henry Soria](https://pierrehenry.be)**. A super passionate & enthusiastic problem-solver engineer. Also a true cheese 🧀, ristretto ☕️, and dark chocolate lover! 😋

[![@phenrysay](https://img.shields.io/badge/x-000000?style=for-the-badge&logo=x)](https://x.com/phenrysay "Follow Me on X") [![pH-7](https://img.shields.io/badge/GitHub-100000?style=for-the-badge&logo=github&logoColor=white)](https://github.com/pH-7 "My GitHub") [![BlueSky](https://img.shields.io/badge/BlueSky-00A8E8?style=for-the-badge&logo=bluesky&logoColor=white)](https://bsky.app/profile/pierrehenry.dev "Follow Me on BlueSky") [![YouTube Video](https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white)](https://www.youtube.com/@pH7Programming "My Channel, NextGen Dev: AI & Code")

## License

**pH7Console** is generously distributed under [MIT](LICENSE.md) license 🎉 Wish you happy, happy productive time! 🤠
