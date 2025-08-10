# pH7Console - ML-First Terminal

> **🚀 An intelligent terminal with local AI capabilities built with Tauri**

pH7Console is a privacy-focused, ML-first terminal that brings the power of local AI to your command line experience. Built with Tauri for maximum performance and security.

## ✨ Features

### 🤖 Local AI Intelligence
- **Command Prediction** - Smart suggestions based on context and history
- **Natural Language Processing** - Convert plain English to shell commands
- **Error Analysis & Fixes** - AI-powered error resolution
- **Output Analysis** - Intelligent parsing and insights from command outputs
- **Smart Completions** - Context-aware command completions

### 🔒 Privacy-First Architecture
- **100% Local Processing** - No data leaves your machine
- **No Telemetry** - Your commands and data stay private
- **Encrypted History** - Local command history encryption
- **Configurable Privacy Levels** - Full control over AI features

### ⚡ Performance Optimized
- **Lightweight Models** - Optimized for MacBook Air and similar hardware
- **Adaptive Loading** - Models load on-demand
- **Battery Aware** - Adjusts AI intensity based on power state
- **Multi-Platform** - Native performance on macOS, Windows, Linux

### 🎯 Productivity Features
- **Multi-Session Management** - Multiple terminal sessions
- **Workflow Automation** - Smart command templates
- **Visual Command Preview** - See what commands will do before execution
- **Session Recording** - Replay and analyze terminal sessions

## 🏗️ Architecture

### Local AI Models (Recommended for MacBook Air)

pH7Console uses **Candle**, a Rust-native ML framework (similar to PyTorch but optimized for inference), to run AI models locally with zero privacy concerns.

#### 🧠 How the Local LLM Works

**Model Architecture**:
- **Framework**: Candle (Rust's equivalent to PyTorch for inference)
- **Quantization**: 4-bit and 8-bit quantized models for efficiency
- **Context Window**: 4K-32K tokens depending on model
- **Inference**: CPU-optimized with optional GPU acceleration

**Supported Models**:
- **Phi-3 Mini (3.8GB)** - Microsoft's efficient instruction-tuned model
  - 3.8B parameters, 4-bit quantized
  - Best balance of capability and performance
  - Specialized for code and reasoning tasks

- **Llama 3.2 1B (1.2GB)** - Meta's ultra-lightweight model
  - 1B parameters, 8-bit quantized
  - Fastest inference on MacBook Air
  - Perfect for basic command suggestions

- **TinyLlama (1.1GB)** - Community's speed-optimized model
  - 1.1B parameters, highly optimized
  - Sub-100ms response times
  - Excellent for real-time completions

- **CodeQwen 1.5B (1.5GB)** - Alibaba's code-specialized model
  - 1.5B parameters, code-focused training
  - Superior understanding of development workflows
  - Best for programming-related tasks

#### 🤖 Learning and Adaptation System

**Unlike PyTorch training, pH7Console uses smart adaptation**:

```rust
// Example of how the system learns (in Rust)
pub struct UserPatternLearner {
    command_embeddings: LocalEmbeddingStore,
    context_analyzer: ContextAnalyzer,
    workflow_detector: WorkflowDetector,
}

impl UserPatternLearner {
    // Learns from command sequences
    pub fn analyze_command_sequence(&mut self, commands: &[Command]) {
        let patterns = self.workflow_detector.find_patterns(commands);
        self.store_workflow_template(patterns);
    }
    
    // Adapts suggestions based on context
    pub fn get_contextual_suggestions(&self, context: &Context) -> Vec<Suggestion> {
        let similar_contexts = self.find_similar_contexts(context);
        self.generate_suggestions(similar_contexts)
    }
}
```

**Learning Mechanisms**:

1. **Command Pattern Recognition**
   - Tracks frequently used command sequences
   - Identifies workflow patterns (e.g., git → test → deploy)
   - Creates smart templates for repeated tasks

2. **Context Understanding**
   - Project type detection (React, Rust, Python, etc.)
   - Working directory analysis
   - File type and structure awareness

3. **Error Pattern Learning**
   - Remembers common mistakes and solutions
   - Builds error-fix knowledge base
   - Suggests preventive measures

4. **Personalization Engine**
   - Learns your preferred command style
   - Adapts verbosity of explanations
   - Customizes suggestion priorities

**Privacy-Preserving Learning**:
```rust
// All learning happens locally
pub struct PrivacyPreservingLearner {
    local_embeddings: HashMap<String, Vec<f32>>,
    encrypted_patterns: EncryptedStorage,
    session_memory: SessionContext,
}

// No data ever leaves your machine
impl PrivacyPreservingLearner {
    pub fn learn_from_session(&mut self, session: &TerminalSession) {
        // Extract patterns without storing raw commands
        let patterns = self.extract_abstract_patterns(session);
        self.update_local_knowledge(patterns);
        // Raw commands are never stored or transmitted
    }
}
```

#### 🚀 Model Performance Comparison

| Model | Size | RAM Usage | Inference Speed | Best For |
|-------|------|-----------|-----------------|----------|
| **Phi-3 Mini** | 3.8GB | 4-6GB | 200-500ms | Complex reasoning, code generation |
| **Llama 3.2 1B** | 1.2GB | 2-3GB | 100-200ms | General commands, explanations |
| **TinyLlama** | 1.1GB | 1.5-2GB | 50-100ms | Real-time completions, quick suggestions |
| **CodeQwen** | 1.5GB | 2-4GB | 150-300ms | Programming tasks, code analysis |

#### 🔧 Model Loading and Optimization

**Adaptive Loading**:
```bash
# Models load based on task complexity
Simple completion → TinyLlama (fast)
Command explanation → Llama 3.2 (balanced)
Code generation → Phi-3 Mini (capable)
Error analysis → CodeQwen (specialized)
```

**Memory Optimization**:
- **Quantization**: 4-bit and 8-bit models reduce memory by 70%
- **Lazy Loading**: Models load only when needed
- **Memory Mapping**: Efficient model storage and access
- **Batch Processing**: Multiple requests processed together

**Battery Optimization**:
- **Power-aware inference**: Adjusts model complexity based on battery level
- **Thermal throttling**: Reduces inference rate if system gets hot
- **Sleep mode**: Models unload during inactivity

### Tech Stack
- **Backend**: Rust + Tauri 2.0
- **AI/ML**: Candle (Rust-native ML framework)
- **Frontend**: React + TypeScript + Tailwind CSS
- **Terminal**: Portable PTY for cross-platform terminal emulation
- **State Management**: Zustand for reactive state

## 🚀 Quick Start

### Prerequisites

#### Required Tools
- **Rust** (1.70+) - [Install Rust](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source ~/.cargo/env
  ```
- **Node.js** (v18+) - [Install Node.js](https://nodejs.org/)
  ```bash
  # Using NVM (recommended)
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
  nvm install --lts && nvm use --lts
  ```
- **Git** - [Install Git](https://git-scm.com/)

#### System Requirements
- **RAM**: 4GB minimum, 8GB recommended (for AI models)
- **Storage**: 5GB free space (for models and dependencies)
- **OS**: macOS 10.15+, Windows 10+, or Linux (Ubuntu 18.04+)

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/EfficientTools/pH7Console.git
   cd pH7Console
   ```

2. **Run the setup script** (handles everything automatically)
   ```bash
   chmod +x setup.sh
   ./setup.sh
   ```

   Or **manual installation**:
   ```bash
   # Install Node.js dependencies
   npm install
   
   # Install Tauri CLI
   npm install -g @tauri-apps/cli@next
   
   # Verify Rust installation
   cd src-tauri && cargo check
   ```

3. **First run** (downloads AI models automatically)
   ```bash
   npm run tauri:dev
   ```

### 🎮 How to Use

#### Starting the Application

**Development Mode** (with hot reload):
```bash
npm run tauri:dev
# or
npx tauri dev
```

**Production Build**:
```bash
npm run tauri:build
./src-tauri/target/release/pH7Console  # Linux/macOS
# or pH7Console.exe on Windows
```

#### Basic Usage

1. **Launch the app** - pH7Console opens with a dark terminal interface
2. **Create sessions** - Click "+" to create new terminal sessions
3. **Natural language commands** - Type what you want in plain English:
   ```
   "show me all large files" 
   → AI suggests: find . -type f -size +100M -exec ls -lh {} \;
   ```
4. **Smart completions** - Start typing and press Tab for AI suggestions
5. **Error assistance** - When commands fail, AI automatically suggests fixes

#### Advanced Features

**Multi-Session Management**:
- `Cmd/Ctrl + T` - New terminal session
- `Cmd/Ctrl + W` - Close current session
- `Cmd/Ctrl + 1-9` - Switch between sessions

**AI Commands**:
- Type naturally: "check git status and stage all changes"
- Use `/explain <command>` to understand any command
- Use `/fix` after an error for AI troubleshooting
- Use `/optimize` to get more efficient alternatives

**Workflow Learning**:
- AI learns your patterns and suggests personalized workflows
- Frequently used command sequences become smart templates
- Context-aware suggestions based on your project type

### Building for Different Platforms

#### Cross-Platform Compilation

**macOS**:
```bash
# Universal binary (Intel + Apple Silicon)
npm run tauri build -- --target universal-apple-darwin

# Intel only
npm run tauri build -- --target x86_64-apple-darwin

# Apple Silicon only  
npm run tauri build -- --target aarch64-apple-darwin
```

**Windows**:
```bash
# Install Windows target (from macOS/Linux)
rustup target add x86_64-pc-windows-msvc

# Build for Windows
npm run tauri build -- --target x86_64-pc-windows-msvc
```

**Linux**:
```bash
# Ubuntu/Debian (.deb)
npm run tauri build -- --target x86_64-unknown-linux-gnu

# AppImage (universal Linux)
npm run tauri build -- --bundles appimage

# RPM (Red Hat/Fedora)
npm run tauri build -- --bundles rpm
```

#### Build Optimization

**Production optimized builds**:
```bash
# Maximum optimization
TAURI_ENV=production npm run tauri build -- --release

# Size optimization
npm run tauri build -- --release --bundles app,dmg --no-default-features
```

**Debug builds** (faster compilation):
```bash
npm run tauri build -- --debug
```

#### Distribution

**macOS**:
- `.dmg` installer in `src-tauri/target/release/bundle/dmg/`
- `.app` bundle in `src-tauri/target/release/bundle/osx/`
- Code signing: Configure in `tauri.conf.json` → `bundle.macOS.signingIdentity`

**Windows**:
- `.msi` installer in `src-tauri/target/release/bundle/msi/`
- `.exe` portable in `src-tauri/target/release/`
- Code signing: Configure certificate in `tauri.conf.json`

**Linux**:
- `.deb` package in `src-tauri/target/release/bundle/deb/`
- `.AppImage` in `src-tauri/target/release/bundle/appimage/`
- `.rpm` package in `src-tauri/target/release/bundle/rpm/`

## 🎮 Usage

## 🎮 Usage

### Natural Language Commands
```
"show me all large files"
→ find . -type f -size +100M -exec ls -lh {} \;

"what's using the most CPU?"
→ top -o cpu

"check git status and stage changes"
→ git status && git add .
```

### Smart Completions
- Type `git` and press **Tab** for context-aware Git commands
- Type `npm` and get project-specific script suggestions
- Type `docker` and get container-aware commands

### AI Features
- **Command Explanation**: Hover over any command for AI explanation
- **Error Recovery**: Automatic error analysis with suggested fixes
- **Context Awareness**: AI understands your project structure and suggests relevant commands
- **Workflow Learning**: AI learns your patterns and suggests optimized workflows

## 🔧 Configuration

### AI Model Settings

Create `~/.ph7console/config.json`:
```json
{
  "ai": {
    "primary_model": "phi3-mini",
    "fallback_model": "tinyllama",
    "temperature": 0.7,
    "max_tokens": 512,
    "privacy_mode": "local_only",
    "learning_enabled": true,
    "context_window": 4096
  },
  "performance": {
    "battery_aware": true,
    "adaptive_loading": true,
    "max_memory_usage": "4GB",
    "inference_timeout": 5000
  },
  "terminal": {
    "default_shell": "/bin/zsh",
    "history_size": 10000,
    "session_persistence": true,
    "auto_suggestions": true
  },
  "ui": {
    "theme": "dark",
    "font_family": "SF Mono",
    "font_size": 14,
    "transparency": 0.95
  }
}
```

### Model Management

**Download specific models**:
```bash
# Download Phi-3 Mini (recommended)
ph7console --download-model phi3-mini

# Download multiple models
ph7console --download-model tinyllama,llama32-1b

# List available models
ph7console --list-models

# Check model status
ph7console --model-status
```

**Model switching**:
```bash
# Switch primary model
ph7console --set-model phi3-mini

# Use specific model for session
ph7console --model tinyllama --session work-session
```

### Advanced Configuration

**Learning and Privacy**:
```json
{
  "learning": {
    "pattern_recognition": true,
    "workflow_detection": true,
    "error_learning": true,
    "personalization": true,
    "data_retention_days": 90
  },
  "privacy": {
    "telemetry": false,
    "local_only": true,
    "encrypt_history": true,
    "auto_cleanup": true,
    "share_anonymous_patterns": false
  }
}
```

**Performance Tuning**:
```json
{
  "inference": {
    "cpu_threads": "auto",
    "memory_limit": "4GB",
    "batch_size": 1,
    "quantization": "4bit",
    "cache_size": "1GB"
  },
  "optimization": {
    "preload_models": ["tinyllama"],
    "lazy_loading": true,
    "memory_mapping": true,
    "thermal_throttling": true
  }
}
```

### Smart Completions
- Type `git` and press **Tab** for context-aware Git commands
- Type `npm` and get project-specific script suggestions
- Type `docker` and get container-aware commands

### AI Features
- **Command Explanation**: Hover over any command for AI explanation
- **Error Recovery**: Automatic error analysis with suggested fixes
- **Context Awareness**: AI understands your project structure and suggests relevant commands
- **Workflow Learning**: AI learns your patterns and suggests optimized workflows

## ⚙️ Configuration

### AI Model Settings
```json
{
  "ai": {
    "model": "phi3-mini",
    "temperature": 0.7,
    "max_tokens": 512,
    "privacy_mode": "local_only"
  }
}
```

### Performance Tuning
```json
{
  "performance": {
    "battery_aware": true,
    "adaptive_loading": true,
    "max_memory_usage": "4GB"
  }
}
```

## 🔧 Development

### Project Structure
```
pH7Console/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── ai/          # AI/ML modules
│   │   ├── terminal/    # Terminal emulation
│   │   └── commands.rs  # Tauri commands
├── src/                 # React frontend
│   ├── components/      # UI components
│   ├── store/          # State management
│   └── types/          # TypeScript types
└── models/             # Local AI models (downloaded)
```

### Adding New AI Capabilities

1. **Define the capability in Rust**:
```rust
#[tauri::command]
pub async fn my_ai_feature(input: String) -> Result<AIResponse, String> {
    // Implementation
}
```

2. **Add to frontend store**:
```typescript
const useAIStore = create((set) => ({
  myFeature: async (input: string) => {
    return await invoke('my_ai_feature', { input });
  }
}));
```

### Testing

```bash
# Run Rust tests
cd src-tauri && cargo test

# Run frontend tests
npm test

# Integration tests
npm run test:e2e
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Add tests: `cargo test` (Rust) and `npm test` (Frontend)
5. Commit with conventional commits: `feat: add amazing feature`
6. Push and submit a pull request

### Code Style
- **Rust**: Use `cargo fmt` and `cargo clippy`
- **TypeScript**: Use Prettier and ESLint
- **Commits**: Follow [Conventional Commits](https://conventionalcommits.org/)

### Testing
```bash
# Backend tests
cd src-tauri && cargo test

# Frontend tests  
npm test

# Integration tests
npm run test:e2e

# AI model tests
cargo test --features ai-tests
```

### Adding New AI Models

1. **Add model configuration**:
```rust
// In src-tauri/src/models/local_llm.rs
LocalModelInfo {
    name: "Your Model Name".to_string(),
    size_mb: 2000,
    model_type: ModelType::Custom,
    capabilities: vec![Capability::CodeGeneration],
    download_url: "huggingface-model-id".to_string(),
    performance_tier: PerformanceTier::Fast,
}
```

2. **Implement model loading**:
```rust
// In src-tauri/src/ai/mod.rs
async fn load_custom_model(&mut self) -> Result<(), Error> {
    // Implementation for your model
}
```

3. **Add tests and documentation**

### Project Guidelines
- **Privacy First**: Never add telemetry or data collection
- **Performance**: Optimize for low-end hardware (MacBook Air)
- **Local Only**: All AI processing must happen locally
- **Cross-Platform**: Ensure compatibility with macOS, Windows, Linux
- **Documentation**: Update README and add inline docs

## 📋 Roadmap

- [ ] **Voice Commands** - Local speech recognition
- [ ] **Plugin System** - Extensible AI capabilities
- [ ] **Team Sharing** - Encrypted workflow sharing
- [ ] **Advanced Visualizations** - Command impact visualization
- [ ] **Multi-Language Support** - Support for multiple languages
- [ ] **Cloud Sync** - Optional encrypted cloud synchronization

## 🐛 Known Issues

- Some AI models may require additional setup on certain hardware
- Terminal themes are currently limited (more coming soon)
- Windows: PTY handling has occasional quirks

## 📄 License

MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Candle** - Rust ML framework
- **Tauri** - Cross-platform app framework
- **Hugging Face** - Pre-trained models
- **The Rust Community** - For amazing crates and support

## 📞 Support

- 💬 **Discussions**: [GitHub Discussions](https://github.com/EfficientTools/pH7Console/discussions)
- 🐛 **Issues**: [GitHub Issues](https://github.com/EfficientTools/pH7Console/issues)
- 📧 **Email**: support@efficienttools.dev

---

**Built with ❤️ for developers who value privacy and productivity**
