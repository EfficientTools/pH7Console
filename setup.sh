#!/bin/bash

echo "🚀 pH7Console Setup Script"
echo "========================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC} $1"
}

print_error() {
    echo -e "${RED}❌${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ️${NC} $1"
}

# Check for required tools
echo "📋 Checking prerequisites..."

# Check for Rust
if ! command -v rustc &> /dev/null; then
    print_error "Rust is not installed"
    echo ""
    print_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    
    if command -v rustc &> /dev/null; then
        print_status "Rust installed successfully: $(rustc --version)"
    else
        print_error "Failed to install Rust. Please install manually from: https://rustup.rs/"
        exit 1
    fi
else
    print_status "Rust found: $(rustc --version)"
fi

# Check for Node.js
if ! command -v node &> /dev/null; then
    print_error "Node.js is not installed"
    echo ""
    print_info "Installation options:"
    echo "1. Official installer: https://nodejs.org/en/download/"
    echo "2. Using nvm: curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash"
    echo "3. Using Homebrew: brew install node"
    echo ""
    
    # Try to install via nvm if available
    if command -v nvm &> /dev/null; then
        print_info "Installing Node.js via nvm..."
        nvm install --lts
        nvm use --lts
    else
        print_warning "Please install Node.js manually and run this script again"
        exit 1
    fi
else
    print_status "Node.js found: $(node --version)"
fi

# Check for npm
if ! command -v npm &> /dev/null; then
    print_error "npm is not installed (should come with Node.js)"
    exit 1
else
    print_status "npm found: $(npm --version)"
fi

echo ""
echo "🔄 Installing dependencies..."

# Install Node.js dependencies
print_info "Installing frontend dependencies..."
if npm install; then
    print_status "Frontend dependencies installed"
else
    print_error "Failed to install frontend dependencies"
    exit 1
fi

# Check Rust setup
print_info "Checking Rust environment..."
cd src-tauri
if cargo check; then
    print_status "Rust environment is ready"
else
    print_error "Rust setup failed"
    exit 1
fi
cd ..

# Install Tauri CLI if not present
if ! command -v cargo-tauri &> /dev/null; then
    print_info "Installing Tauri CLI..."
    if cargo install tauri-cli --version "^2.0.0"; then
        print_status "Tauri CLI installed"
    else
        print_warning "Failed to install Tauri CLI globally, using npx instead"
    fi
fi

# Setup Rust targets
print_info "Setting up Rust targets..."
rustup target add $(rustc -vV | sed -n 's|host: ||p')

# Create config directory
print_info "Setting up configuration..."
mkdir -p ~/.ph7console
if [ ! -f ~/.ph7console/config.json ]; then
    cat > ~/.ph7console/config.json << EOF
{
  "ai": {
    "primary_model": "phi3-mini",
    "fallback_model": "tinyllama",
    "temperature": 0.7,
    "max_tokens": 512,
    "privacy_mode": "local_only",
    "learning_enabled": true
  },
  "performance": {
    "battery_aware": true,
    "adaptive_loading": true,
    "max_memory_usage": "4GB"
  },
  "terminal": {
    "default_shell": "/bin/zsh",
    "history_size": 10000,
    "session_persistence": true
  }
}
EOF
    print_status "Default configuration created"
fi

echo ""
print_status "Setup complete!"
echo ""
echo "🚀 Quick start commands:"
echo "  Development: ${BLUE}npm run tauri:dev${NC}"
echo "  Build:       ${BLUE}npm run tauri:build${NC}"
echo "  Test:        ${BLUE}cargo test${NC} (in src-tauri directory)"
echo ""
echo "📚 Next steps:"
echo "1. Run '${BLUE}npm run tauri:dev${NC}' to start development server"
echo "2. The app will automatically download AI models on first run"
echo "3. Check README.md for detailed usage instructions"
echo ""
echo "🎉 Happy coding with pH7Console!"

# Test the setup
echo ""
print_info "Testing setup..."
if npm run tauri build --help &> /dev/null; then
    print_status "Build system is working"
else
    print_warning "Build system test failed, but setup should still work"
fi
