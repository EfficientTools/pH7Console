# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

pH7Console is a privacy-first, AI-powered terminal application built with:
- **Frontend**: React 18 + TypeScript + Tailwind CSS
- **Backend**: Rust (Tauri 2.0) for native performance
- **AI/ML**: Local machine learning with learning engine and pattern recognition
- **Terminal**: Cross-platform PTY with custom terminal emulation

This is a hybrid application where the Rust backend handles terminal operations, AI/ML processing, and system interactions, while the React frontend provides the UI.

## Common Development Commands

### Setup and Installation
```bash
# First-time setup (installs all dependencies)
./setup.sh

# Manual dependency installation
npm install                          # Frontend dependencies
cd src-tauri && cargo check          # Rust dependencies
```

### Development
```bash
# Start development server with hot reload
npm run tauri:dev

# Frontend only (for UI development)
npm run dev

# Build TypeScript without running
npm run build
```

### Testing
```bash
# Run all tests
npm run test

# Unit tests (Vitest)
npm run test:unit
npm run test:unit:watch             # Watch mode
npm run test:unit:ui                # UI mode

# Integration tests (Playwright)
npm run test:integration
npm run test:integration:headed     # With browser UI
npm run test:integration:debug      # Debug mode

# Rust tests
npm run test:rust
cd src-tauri && cargo test          # Alternative
cd src-tauri && cargo test -- --nocapture  # With output

# Run single Rust test
cd src-tauri && cargo test test_name
```

### Linting and Formatting
```bash
# TypeScript/React linting
npm run lint

# Rust formatting and linting
cd src-tauri && cargo fmt
cd src-tauri && cargo clippy
```

### Building
```bash
# Production build
npm run tauri:build

# Build for specific platforms
npm run tauri:build -- --target universal-apple-darwin  # macOS universal
npm run tauri:build -- --target x86_64-pc-windows-msvc  # Windows
npm run tauri:build -- --target x86_64-unknown-linux-gnu  # Linux

# App Store build (macOS)
npm run tauri:build:appstore
npm run tauri:upload:appstore
npm run appstore                    # Build and upload
```

### Cleanup
```bash
# Clean all build artifacts and reinstall
npm run clean
```

## Architecture

### High-Level Structure

```
Frontend (React/TypeScript) <-> Tauri IPC <-> Backend (Rust)
         └─ UI Components              └─ Terminal/PTY
         └─ State Management              └─ AI/ML Engine
         └─ Terminal Renderer             └─ System Integration
```

### Backend Architecture (Rust)

**Main Entry Point**: `src-tauri/src/main.rs`
- Initializes `AppState` with two main managers:
  - `ModelManager`: Handles AI/ML operations
  - `TerminalManager`: Manages terminal sessions and PTY

**Key Modules**:
- `ai/` - AI and machine learning functionality
  - `mod.rs`: Core AI processing, model management, and ML inference
  - `learning_engine.rs`: Pattern learning, user behavior analysis, workflow detection
  - `enhanced_context.rs`: Context extraction and analysis (git, project type, file structure)
  - `agent.rs`: AI agent for complex tasks and command suggestions

- `terminal/` - Terminal emulation and PTY handling
  - Cross-platform terminal session management
  - Command execution and output streaming

- `models/` - AI model definitions and configurations
  - Local LLM model information and loading logic

- `commands.rs` - Tauri command handlers (IPC bridge between frontend and backend)
  - All functions exposed to frontend via `invoke()`
  - Terminal operations, AI commands, file system operations

**State Management**:
The `AppState` is shared across all commands via Tauri's state management:
```rust
pub struct AppState {
    pub model_manager: Arc<Mutex<ModelManager>>,
    pub terminal_manager: Arc<Mutex<TerminalManager>>,
}
```

### Frontend Architecture (React/TypeScript)

**Entry Point**: `src/main.tsx` -> `src/App.tsx`

**State Management** (`src/store/`):
- `terminalStore.ts`: Zustand store for terminal sessions, command history, active sessions
- `aiStore.ts`: AI-related state including suggestions, model status, learning data

**Key Components** (`src/components/`):
- `Terminal.tsx`: Main terminal component with xterm.js integration
- `TerminalHeader.tsx`: Session tabs and terminal controls
- `AIPanel.tsx`: AI assistance panel with command suggestions and explanations
- `AgentPanel.tsx`: AI agent interface for complex tasks
- `SmartSuggestions.tsx`: Context-aware command completion suggestions
- `FileExplorer.tsx`: File system navigation integrated with terminal
- `Sidebar.tsx`: Application sidebar with session management
- `Settings.tsx`: Application settings and preferences
- `HistoryModal.tsx`: Command history viewer and search

**AI Engine** (`src/ai/`):
- `IntelligentPredictionEngine.ts`: Frontend prediction engine that works with backend ML
- Pattern recognition and confidence scoring
- Context-aware suggestion generation

### Communication Flow

1. **User Input** -> Terminal Component
2. **Frontend Processing** -> Check for natural language or special commands
3. **Tauri Invoke** -> Call backend via `invoke('command_name', { params })`
4. **Rust Processing** -> Command handler in `commands.rs`
5. **Business Logic** -> Terminal manager or AI manager processes request
6. **Response** -> Returns to frontend via Promise
7. **UI Update** -> React state updates and re-renders

### AI/ML Learning System

The learning engine operates entirely locally:
- **Pattern Recognition**: Learns command patterns from user behavior
- **Context Analysis**: Understands working directory, git repos, project types
- **Workflow Detection**: Recognizes command sequences and creates templates
- **Error Learning**: Remembers successful error resolutions
- **Session Memory**: Maintains context within and across sessions

All learning data is stored locally and encrypted.

## Important Conventions

### Privacy and Security
- **Never** add telemetry or external API calls for core functionality
- All AI/ML processing must happen locally
- No user data should ever leave the machine
- Encrypt sensitive data in local storage

### Code Style
- **Rust**: Use `cargo fmt` and follow `cargo clippy` suggestions
- **TypeScript**: Follow ESLint rules, use strict typing (no `any`)
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`

### Testing Requirements
- Add Rust tests for new backend functionality
- Add Vitest tests for new frontend utilities
- Add Playwright tests for new user flows
- Run full test suite before submitting PRs

### Performance Considerations
- Target low-end hardware (MacBook Air)
- Models should be <5GB
- Optimize for battery life on laptops
- Use lazy loading for AI models
- Keep terminal rendering performant (smooth 60fps)

## Adding New Features

### Adding a New Tauri Command
1. Define command in `src-tauri/src/commands.rs`:
```rust
#[tauri::command]
pub async fn my_command(state: State<'_, AppState>, param: String) -> Result<String, String> {
    // Implementation
    Ok("result".to_string())
}
```

2. Register in `src-tauri/src/main.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    commands::my_command,
])
```

3. Call from frontend:
```typescript
import { invoke } from '@tauri-apps/api/tauri';
const result = await invoke<string>('my_command', { param: 'value' });
```

### Adding AI Capabilities
1. Implement in `src-tauri/src/ai/mod.rs` or related module
2. Expose via Tauri command in `commands.rs`
3. Add to AI store or create new store if needed
4. Integrate into UI components

### Adding New UI Components
1. Create component in `src/components/`
2. Use TypeScript with strict typing
3. Follow existing component patterns (functional components with hooks)
4. Use Tailwind for styling
5. Add to relevant parent component or route

## Configuration

**User Configuration**: `~/.ph7console/config.json`
- AI model preferences
- Performance settings
- Terminal preferences
- UI customization

**Tauri Configuration**: `src-tauri/tauri.conf.json`
- App metadata
- Build settings
- Platform-specific configurations
- Permissions and security settings

## Troubleshooting

**Models not loading**: Ensure sufficient disk space (5GB+) and check logs for download errors

**PTY issues on Windows**: Windows PTY handling has quirks; check terminal module for platform-specific code

**Build errors**: Try `cargo clean` in src-tauri/ and `npm run clean` at root

**Tests failing**: Ensure all dependencies are installed and models are available if testing AI features
