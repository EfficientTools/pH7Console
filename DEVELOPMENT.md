# pH7Console Development Guide

## 🛠️ Prerequisites Installation

Since this is a fresh system, you'll need to install the following tools:

### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Install Node.js (Choose one method)

**Option A: Official Installer (Recommended)**
1. Visit https://nodejs.org/
2. Download the LTS version for macOS
3. Run the installer

**Option B: Using NVM (Node Version Manager)**
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.zshrc
nvm install --lts
nvm use --lts
```

**Option C: Using Homebrew (if you have admin access)**
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install node
```

### 3. Verify Installation
```bash
# Check versions
rust --version    # Should show 1.70+
node --version    # Should show v18+
npm --version     # Should show 9+
```

## 🚀 Project Setup

Once you have the prerequisites:

### 1. Install Dependencies
```bash
cd pH7Console
npm install
```

### 2. Install Tauri CLI
```bash
# Global installation
npm install -g @tauri-apps/cli@next

# Or use npx (no global install needed)
npx @tauri-apps/cli@next --help
```

### 3. Install Rust Dependencies
```bash
cd src-tauri
cargo check  # This will download Rust dependencies
```

## 🏃‍♂️ Running the Project

### Development Mode
```bash
# Using global Tauri CLI
npm run tauri dev

# Or using npx
npx tauri dev
```

### Building for Production
```bash
# Build for current platform
npm run tauri build

# Build universal binary for macOS
npm run tauri build -- --target universal-apple-darwin
```

## 🧪 Testing

### Rust Backend Tests
```bash
cd src-tauri
cargo test
```

### Frontend Tests (once dependencies are installed)
```bash
npm test
```

## 📁 Project Structure Overview

```
pH7Console/
├── 📄 README.md              # Main documentation
├── 📄 package.json           # Node.js dependencies
├── 📄 Cargo.toml            # Rust dependencies (in src-tauri/)
├── 🗂️ src/                   # React frontend
│   ├── 🗂️ components/        # UI components
│   ├── 🗂️ store/            # State management
│   └── 📄 App.tsx           # Main app component
├── 🗂️ src-tauri/            # Rust backend
│   ├── 📄 Cargo.toml        # Rust dependencies
│   ├── 🗂️ src/              # Rust source code
│   │   ├── 📄 main.rs       # Main entry point
│   │   ├── 🗂️ ai/           # AI/ML modules
│   │   ├── 🗂️ terminal/     # Terminal emulation
│   │   └── 📄 commands.rs   # Tauri commands
│   └── 📄 tauri.conf.json   # Tauri configuration
└── 🗂️ models/               # AI models (downloaded at runtime)
```

## 🤖 AI Models

The application will automatically download lightweight AI models on first run:

- **Phi-3 Mini (3.8GB)** - Primary model for most tasks
- **Llama 3.2 1B (1.2GB)** - Backup lightweight model
- **TinyLlama (1.1GB)** - Ultra-fast for basic completions

Models are stored locally and never sent to external servers.

## 🎯 Key Features to Test

1. **Smart Command Suggestions** - Type commands and see AI completions
2. **Natural Language Processing** - Describe what you want in plain English
3. **Error Analysis** - Run commands that fail and get AI-powered fixes
4. **Multi-Session Management** - Create multiple terminal sessions
5. **Command History Intelligence** - AI learns from your command patterns

## 🐛 Troubleshooting

### Common Issues

**"npm not found"**
- Install Node.js first (see prerequisites above)

**"cargo not found"**
- Install Rust first (see prerequisites above)

**"permission denied"**
- Make sure you have proper permissions or use `sudo` where needed

**Build errors**
- Try: `cargo clean` in src-tauri/ then rebuild
- Update Rust: `rustup update`

**AI models not loading**
- Check internet connection for initial download
- Ensure sufficient disk space (5GB+ recommended)

## 🔧 Development Tips

1. **Hot Reload**: The frontend supports hot reload during development
2. **Rust Changes**: Restart `tauri dev` when modifying Rust code
3. **Debugging**: Use browser dev tools for frontend, `println!` for Rust
4. **Testing**: Run tests frequently with `cargo test`

## 📞 Getting Help

- 📖 Check the main README.md for detailed information
- 🐛 Report issues on GitHub
- 💬 Join discussions in the project repository

Happy coding with pH7Console! 🚀
