# BigBot - Advanced Mumble Voice Chat Bot

[![Rust](https://github.com/codeandkey/bigbot/actions/workflows/ci.yml/badge.svg)](https://github.com/codeandkey/bigbot/actions/workflows/ci.yml)
[![Release](https://github.com/codeandkey/bigbot/actions/workflows/release.yml/badge.svg)](https://github.com/codeandkey/bigbot/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

BigBot is a feature-rich, high-performance Mumble voice chat bot written in Rust. It provides sound management, user greetings/farewells, command aliases, and extensive customization options through a comprehensive YAML configuration system.

## 🎯 Features

### 🔊 **Sound Management System**
- **Play sounds** from a curated database with simple commands
- **Pull audio** directly from YouTube, Twitter, and other URLs
- **Smart sound organization** with automatic file management
- **User-specific greetings** and farewells with custom commands
- **Random sound playback** for variety and entertainment

### 🤖 **Advanced Bot Behavior**
- **3-mode greeting/farewell system**: `all` (custom + random fallback), `custom` (only user-set), or `none` (silent)
- **Username-based user settings** for persistent personalization
- **Command aliases** for complex command sequences
- **Private message support** with configurable access control
- **Session management** with automatic reconnection

### ⚙️ **Comprehensive Configuration**
- **YAML-based configuration** with automatic generation
- **Command-line overrides** for quick adjustments
- **Hot-reload capabilities** for configuration changes
- **Path customization** for data, certificates, and sounds
- **Server-specific settings** with SSL/TLS support

### 🔧 **Developer-Friendly**
- **Async/await throughout** for high performance
- **Modular command system** for easy extension
- **SQLite database** for reliable data persistence
- **Comprehensive logging** with configurable verbosity
- **Error handling** with detailed diagnostics

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** (latest stable recommended)
- **FFmpeg** (for audio processing and URL pulling)
- **SQLite** (included with most systems)

### Installation

#### Option 1: Download from Releases
```bash
# Download the latest release for your platform
wget https://github.com/codeandkey/bigbot/releases/latest/download/bigbot-linux-x86_64.tar.gz
tar -xzf bigbot-linux-x86_64.tar.gz
chmod +x bigbot
```

#### Option 2: Build from Source
```bash
# Clone the repository
git clone https://github.com/codeandkey/bigbot.git
cd bigbot

# Build the project
cargo build --release

# The binary will be available at target/release/bigbot
```

### First Run

```bash
# Run with default settings (creates ~/.bigbot/config.yml automatically)
./bigbot

# Or specify a custom configuration location
./bigbot --config /path/to/config.yml

# Enable verbose logging
./bigbot --verbose
```

The bot will automatically:
1. Create the configuration directory (`~/.bigbot`)
2. Generate a fully documented configuration file
3. Set up the SQLite database
4. Generate SSL certificates for secure connections

## 📖 Configuration

### Configuration File Structure

Big Bot uses a comprehensive YAML configuration system:

```yaml
# Bot-specific settings
bot:
  username: "Big Bot"
  password: null  # Optional server password
  verbose: false

# Mumble server connection
server:
  host: "localhost"
  port: 64738
  timeout_seconds: 10

# Bot behavior settings
behavior:
  # Options: "all", "custom", "none"
  auto_greetings: all
  auto_farewells: custom
  allow_private_commands: true

# File and directory paths
paths:
  data_dir: null  # Defaults to ~/.bigbot
  cert_file: null
  key_file: null
  trusted_certs_dir: null
```

### Behavior Modes

The new **3-mode system** provides granular control:

- **`all`**: Play custom sounds when available, fallback to random sounds
- **`custom`**: Only play user-set custom sounds (silent if none exists)
- **`none`**: Completely silent mode

### Command Line Options

```bash
# Configuration
./bigbot --config /path/to/config.yml
./bigbot --data-dir /custom/data/path

# Logging
./bigbot --verbose

# Help
./bigbot --help
```

## 🎮 Commands

### Sound Management

```bash
# Play sounds
!sounds play                    # Random sound
!sounds play abc123            # Specific sound by code
!sounds list                   # Show all available sounds
!sounds info abc123            # Detailed sound information

# Pull audio from URLs
!sounds pull https://youtube.com/watch?v=... 1:30 5    # 5 seconds starting at 1:30

# File management
!sounds scan                   # Find orphaned sound files
```

### User Personalization

```bash
# Set personal greetings
!greeting sounds play welcome     # Play 'welcome' sound when you join
!greeting                         # Remove your current greeting

# Set personal farewells
!farewell sounds play goodbye     # Play 'goodbye' sound when you leave
!farewell                         # Remove your current farewell
```

### Command Aliases

```bash
# Create command shortcuts
!alias myhello sounds play hello
!alias party sounds play party; sounds play music
!alias list                      # Show all available aliases
!alias delete myhello            # Remove an alias
!alias search mamamia            # Search existing aliases

# Use aliases
!myhello                        # Executes the aliased command
```

### Utility Commands

```bash
!ping                           # Test bot responsiveness
!bind <command>                 # Set user-specific bind command
```

## 🏗️ Architecture

### Core Components

- **Session Management** (`src/session.rs`): Handles Mumble protocol and user interactions
- **Command System** (`src/commands/`): Modular command processing with extensible architecture
- **Audio Processing** (`src/audio.rs`): Sound mixing and playback with async streaming
- **Database Layer** (`src/database/`): SQLite with Sea-ORM for data persistence
- **Configuration** (`src/config.rs`): YAML-based config with automatic generation

### Key Technologies

- **Async Rust** with Tokio runtime for high concurrency
- **Protobuf** for Mumble protocol communication
- **RustTLS** for secure connections
- **Sea-ORM** for type-safe database operations
- **Serde** for configuration serialization
- **Clap** for command-line interface

## 🔧 Development

### Building

```bash
# Standard build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Run with development logging
RUST_LOG=debug cargo run
```

### Adding Commands

Big Bot uses a modular command system. To add a new command:

1. Create a new file in `src/commands/`
2. Implement the `Command` trait
3. Register the command in `src/commands/mod.rs`

Example:
```rust
use super::{Command, SessionTools, CommandContext};
use crate::error::Error;

#[derive(Default)]
pub struct MyCommand;

#[async_trait::async_trait]
impl Command for MyCommand {
    async fn execute(&mut self, tools: &dyn SessionTools, context: CommandContext, args: Vec<String>) -> Result<(), Error> {
        tools.reply("Hello from my custom command!").await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "mycommand"
    }
    
    fn description(&self) -> &str {
        "An example custom command"
    }
}
```

### Database Schema

The bot uses SQLite with the following main tables:
- `sounds`: Sound file metadata and storage information
- `user_settings`: Per-user greeting/farewell commands  
- `aliases`: User-defined command shortcuts

## 🐳 Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ffmpeg ca-certificates
COPY --from=builder /app/target/release/bigbot /usr/local/bin/
ENTRYPOINT ["bigbot"]
```

```bash
# Build and run
docker build -t big-bot .
docker run -v $HOME/.bigbot:/root/.bigbot big-bot --config /root/.bigbot/config.yml
```

## 📊 Performance

Big Bot is designed for high performance and reliability:

- **Low Memory Footprint**: ~10-20MB RAM usage
- **Fast Startup**: <2 seconds to full functionality
- **Concurrent Audio**: Multiple simultaneous sound streams
- **Database Efficiency**: Optimized queries with connection pooling
- **Error Recovery**: Automatic reconnection and graceful degradation

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone and setup
git clone https://github.com/codeandkey/bigbot.git
cd bigbot

# Install development dependencies
cargo install cargo-watch
cargo install cargo-edit

# Run in development mode with hot reload
cargo watch -x 'run -- --verbose'
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test config

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

## 📋 Roadmap

- [ ] **Web Dashboard**: Browser-based configuration and monitoring
- [ ] **Plugin System**: Dynamic command loading without restarts
- [ ] **Music Streaming**: Direct integration with Spotify/YouTube Music
- [ ] **Voice Commands**: Speech recognition for hands-free control
- [ ] **Multi-Server**: Connect to multiple Mumble servers simultaneously
- [ ] **REST API**: External integration and control interface

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

### Getting Help

- **Documentation**: Check the [CONFIG.md](CONFIG.md) for detailed configuration options
- **Issues**: Report bugs or request features on [GitHub Issues](https://github.com/codeandkey/bigbot/issues)
- **Discussions**: Join the community on [GitHub Discussions](https://github.com/codeandkey/bigbot/discussions)

### Troubleshooting

#### Common Issues

**Bot won't connect to server:**
```bash
# Check server details in config
cat ~/.bigbot/config.yml

# Test with verbose logging
./bigbot --verbose
```

**Sounds not playing:**
```bash
# Verify FFmpeg installation
ffmpeg -version

# Check sound files
!sounds scan
```

**Configuration errors:**
```bash
# Regenerate configuration
rm ~/.bigbot/config.yml
./bigbot  # Will auto-generate new config
```

## 🎉 Acknowledgments

- **Mumble Team** for the excellent voice chat protocol
- **Rust Community** for the amazing ecosystem
- **Contributors** who help make Big Bot better

---

**Made with ❤️ and 🦀 Rust**
