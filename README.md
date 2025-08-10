# pH7Console: An AI-Powered Terminal that Respects Your Privacy

pH7Console is a privacy-focused, ML-first terminal that brings the power of local AI to your command line experience. Built with Tauri for maximum performance and security.

At the moment, it includes the following AI capabilities 🚀

- **Local LLM Processing** - Phi-3 Mini, Llama 3.2, TinyLlama, CodeQwen
- **Natural Language Commands** - Convert plain English to shell commands
- **Smart Error Resolution** - AI-powered troubleshooting and fixes
- **Context-Aware Suggestions** - Intelligent command completions
- **Privacy-Preserving Learning** - Adapts to your workflow without data collection

## Key Features

### 🤖 Local AI Intelligence
- **Command Prediction** - Smart suggestions based on context and history
- **Natural Language Processing** - Convert plain English to shell commands  
- **Error Analysis & Fixes** - AI-powered error resolution
- **Smart Completions** - Context-aware command completions

### 🔒 Privacy-First Architecture  
- **100% Local Processing** - No data leaves your machine
- **No Telemetry** - Your commands and data stay private
- **Encrypted History** - Local command history encryption

### ⚡ Performance Optimized
- **Lightweight Models** - Optimized for MacBook Air and similar hardware
- **Adaptive Loading** - Models load on-demand
- **Multi-Platform** - Native performance on macOS, Windows, Linux

### 🎯 Productivity Features
- **Multi-Session Management** - Multiple terminal sessions
- **Workflow Automation** - Smart command templates  
- **Session Recording** - Replay and analyze terminal sessions

### Utility

- `cargo fmt` (in src-tauri/) - Format Rust code
- `cargo clippy` (in src-tauri/) - Lint Rust code for common mistakes
- `npm run lint` - Check TypeScript/React code style
- `npm run type-check` - Verify TypeScript types

## How to Use

### Basic Usage

1. **Launch the app** - pH7Console opens with a dark terminal interface
2. **Natural language commands** - Type what you want in plain English:
   ```
   "show me all large files" 
   → AI suggests: find . -type f -size +100M -exec ls -lh {} \;
   ```
3. **Smart completions** - Start typing and press Tab for AI suggestions
4. **Error assistance** - When commands fail, AI automatically suggests fixes

### AI Features
- **Command Explanation**: Hover over any command for AI explanation
- **Error Recovery**: Automatic error analysis with suggested fixes  
- **Context Awareness**: AI understands your project structure
- **Workflow Learning**: AI learns your patterns and suggests optimized workflows

## Technical Stack

- **Frontend**: React 18 + TypeScript + Tailwind CSS
- **Backend**: Rust (Tauri 2.0) for native performance  
- **AI Runtime**: Candle ML framework (Rust-native)
- **Terminal**: Cross-platform PTY with xterm.js

### Local AI Models

pH7Console runs AI models locally using **Candle** (Rust's ML framework) for zero privacy concerns:

- **Phi-3 Mini (3.8GB)** - Best for command understanding
- **Llama 3.2 1B (1.2GB)** - Fastest responses on MacBook Air
- **TinyLlama (1.1GB)** - Minimal resource usage  
- **CodeQwen (1.5GB)** - Specialized for code tasks

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate and follow the existing code style.

## Author

**Pierre-Henry Soria**

- 🌐 Website: [ph7.me](https://ph7.me)
- 🐙 GitHub: [@pH-7](https://github.com/pH-7)  
- 🐦 Twitter: [@phenrysoria](https://twitter.com/phenrysoria)
- 📧 Email: hi+github{{AT}}ph7.me

*Passionate about creating developer tools that respect privacy and enhance productivity.*

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

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

## Get Started

1. **Clone the repository**
   ```bash
   git clone https://github.com/EfficientTools/pH7Console.git
   cd pH7Console
   ```

2. **Install dependencies** - Run the automated setup script
   ```bash
   chmod +x setup.sh
   ./setup.sh
   ```

3. **Start development** - Launch the terminal with AI capabilities
   ```bash
   npm run tauri:dev
   ```

### Other Commands

- `npm run tauri:build` - Build for production (creates native binaries)
- `npm run tauri:build -- --target universal-apple-darwin` - Build universal macOS binary
- `npm run tauri:build -- --target x86_64-pc-windows-msvc` - Build for Windows
- `npm run test` - Run all tests (Rust backend and TypeScript frontend)
- `npm run clean` - Clean all build artifacts and reinstall dependencies

## Prerequisites

**System Requirements:**
- **RAM**: 4GB minimum, 8GB recommended (for AI models)
- **Storage**: 5GB free space (for models and dependencies)  
- **OS**: macOS 10.15+, Windows 10+, or Linux (Ubuntu 18.04+)

**Required Tools:**
- **Rust** (1.70+) - [Install Rust](https://rustup.rs/)
- **Node.js** (v18+) - [Install Node.js](https://nodejs.org/)
- **Git** - [Install Git](https://git-scm.com/)

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

## Author

[![Pierre-Henry Soria](https://avatars0.githubusercontent.com/u/1325411?s=200)](https://ph7.me "Pierre-Henry Soria, Software Developer")

Made with ❤️ by **[Pierre-Henry Soria](https://pierrehenry.be)**. A super passionate & enthusiastic Problem-Solver / Senior Software Engineer. Also a true cheese 🧀, ristretto ☕️, and dark chocolate lover! 😋

[![@phenrysay](https://img.shields.io/badge/x-000000?style=for-the-badge&logo=x)](https://x.com/phenrysay "Follow Me on X")  [![pH-7](https://img.shields.io/badge/GitHub-100000?style=for-the-badge&logo=github&logoColor=white)](https://github.com/pH-7 "My GitHub")  [![LinkedIn](https://img.shields.io/badge/LinkedIn-0077B5?style=for-the-badge&logo=linkedin&logoColor=white)](https://www.linkedin.com/in/ph7enry/ "LinkedIn Profile")

# About the Project

**pH7Console** is part of the challenge `#Privacy-First-AI-Tools`, a collection of **innovative AI projects** focused on bringing artificial intelligence capabilities to developers while maintaining complete privacy and data sovereignty. Built with a commitment to local processing and zero telemetry. Hope you enjoy 🤗

Feel free to connect, and reach me at **[my LinkedIn Profile](https://www.linkedin.com/in/ph7enry/)** 🚀

## License

Distributed under [MIT](https://opensource.org/license/mit) license 🎉 Wish you happy, happy coding! 🤠
