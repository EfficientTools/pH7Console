# Contributing to pH7Console

We love your input! We want to make contributing to pH7Console as easy and transparent as possible, whether it's:

- Reporting a bug
- Discussing the current state of the code
- Submitting a fix
- Proposing new features
- Becoming a maintainer

## 🚀 Development Process

We use GitHub to host code, track issues and feature requests, and accept pull requests.

### Pull Request Process

1. **Fork** the repository and create your branch from `main`
2. **Clone** your fork locally
3. **Install** dependencies: `./setup.sh`
4. **Create** a feature branch: `git checkout -b feature/amazing-feature`
5. **Make** your changes
6. **Test** your changes thoroughly
7. **Commit** using conventional commits
8. **Push** to your fork and submit a pull request

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Adding tests

## 📝 Commit Convention

We use [Conventional Commits](https://conventionalcommits.org/) for clear and structured commit messages:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that don't affect code meaning (formatting, etc.)
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvements
- `test`: Adding missing tests
- `chore`: Changes to build process or auxiliary tools

### Examples
```bash
feat(ai): add natural language command translation
fix(terminal): resolve PTY handling on Windows
docs(readme): update installation instructions
perf(models): optimize model loading performance
```

## 🧪 Testing

### Running Tests

**Backend (Rust)**:
```bash
cd src-tauri
cargo test
cargo test --features ai-tests  # AI-specific tests
cargo clippy                    # Linting
cargo fmt                       # Formatting
```

**Frontend (TypeScript)**:
```bash
npm test                        # Jest tests
npm run lint                    # ESLint
npm run type-check              # TypeScript checking
```

**Integration Tests**:
```bash
npm run test:e2e               # End-to-end tests
```

### Writing Tests

**Rust Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_response_generation() {
        // Test implementation
    }

    #[tokio::test]
    async fn test_async_function() {
        // Async test implementation
    }
}
```

**TypeScript Tests**:
```typescript
describe('Terminal Component', () => {
  test('should render correctly', () => {
    // Test implementation
  });
});
```

## 🎯 Code Style

### Rust
- Use `cargo fmt` for formatting
- Follow `cargo clippy` suggestions
- Write documentation comments for public APIs
- Use meaningful variable names
- Follow Rust naming conventions

### TypeScript/React
- Use Prettier for formatting
- Follow ESLint rules
- Use TypeScript strictly (no `any` types)
- Write JSDoc comments for complex functions
- Use functional components with hooks

### General Guidelines
- Write self-documenting code
- Add comments for complex logic
- Keep functions small and focused
- Use descriptive commit messages
- Update documentation for new features

## 🔒 Privacy and Security

pH7Console is privacy-first. When contributing:

- **Never** add telemetry or data collection
- **All** AI processing must happen locally
- **No** external API calls for core functionality
- **Encrypt** sensitive data storage
- **Document** privacy implications of changes

## 🤖 AI Model Contributions

### Adding New Models

1. **Research** model licensing and compatibility
2. **Add** model configuration:
```rust
LocalModelInfo {
    name: "Model Name".to_string(),
    size_mb: 2000,
    model_type: ModelType::YourModel,
    capabilities: vec![Capability::CodeGeneration],
    download_url: "huggingface-repo".to_string(),
    performance_tier: PerformanceTier::Fast,
}
```

3. **Implement** model loading logic
4. **Add** comprehensive tests
5. **Update** documentation
6. **Verify** performance on MacBook Air

### Model Requirements
- Must run locally (no cloud dependencies)
- Size < 5GB for primary models
- Support CPU inference
- Compatible with Candle framework
- Permissive license (Apache, MIT, etc.)

## 📋 Issue Reporting

### Bug Reports

Use the bug report template and include:

- **OS and version** (macOS 13.0, Windows 11, Ubuntu 22.04)
- **Hardware specs** (especially for AI model issues)
- **Steps to reproduce**
- **Expected vs actual behavior**
- **Console logs** (if applicable)
- **Screenshots** (if UI related)

### Feature Requests

- **Describe** the problem you're solving
- **Explain** why this would be useful
- **Consider** privacy implications
- **Think** about performance impact
- **Suggest** implementation approach

## 🎨 UI/UX Guidelines

### Design Principles
- **Dark theme first** (terminal users prefer dark interfaces)
- **Minimize visual clutter** (focus on content)
- **Keyboard-first interaction** (terminal users love keyboards)
- **Consistent spacing** (use Tailwind spacing scale)
- **Accessible colors** (proper contrast ratios)

### Component Guidelines
- Use TypeScript interfaces for props
- Follow React best practices
- Keep components focused and reusable
- Use semantic HTML
- Add proper ARIA labels

## 🔧 Development Environment

### Recommended Tools
- **VS Code** with Rust and TypeScript extensions
- **Rust Analyzer** for Rust development
- **Prettier** and **ESLint** for TypeScript
- **Git** with conventional commit tools

### VS Code Extensions
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tauri-apps.tauri-vscode",
    "bradlc.vscode-tailwindcss",
    "esbenp.prettier-vscode",
    "ms-vscode.vscode-typescript-next"
  ]
}
```

## 📚 Documentation

### Code Documentation
- **Rust**: Use `///` for public APIs
- **TypeScript**: Use JSDoc comments
- **Keep** examples up to date
- **Document** complex algorithms

### README Updates
- Update installation instructions if needed
- Add new features to feature list
- Update screenshots if UI changes
- Keep configuration examples current

## 🚦 Release Process

1. **Version bump** in `Cargo.toml` and `package.json`
2. **Update** CHANGELOG.md
3. **Test** on all platforms
4. **Create** release PR
5. **Tag** release after merge
6. **Build** and publish artifacts

## 💬 Community

### Code of Conduct
- **Be respectful** and inclusive
- **Help** newcomers learn
- **Focus** on constructive feedback
- **Celebrate** contributions

### Getting Help
- 📖 Check existing documentation
- 🔍 Search existing issues
- 💬 Join GitHub discussions
- 📧 Email maintainers for private issues

## 🎉 Recognition

Contributors are recognized through:
- **Git history** (your commits are forever!)
- **Release notes** (major contributions highlighted)
- **README credits** (significant contributors listed)
- **Social media** shoutouts for major features

---

## Quick Start for Contributors

```bash
# 1. Fork and clone
git clone https://github.com/yourusername/pH7Console.git
cd pH7Console

# 2. Set up development environment
./setup.sh

# 3. Create feature branch
git checkout -b feature/amazing-feature

# 4. Make changes and test
npm run tauri:dev
cargo test

# 5. Commit and push
git commit -m "feat: add amazing feature"
git push origin feature/amazing-feature

# 6. Create pull request
```

Thank you for contributing to pH7Console! 🚀
