# BigBot - Advanced Mumble Voice Chat Bot

[![Rust](https://github.com/codeandkey/bigbot/actions/workflows/ci.yml/badge.svg)](https://github.com/codeandkey/bigbot/actions/workflows/ci.yml)
[![Release](https://github.com/codeandkey/bigbot/actions/workflows/release.yml/badge.svg)](https://github.com/codeandkey/bigbot/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance Mumble voice chat bot written in Rust with sound management, user personalization, and extensive customization options.

## Features

- **Sound Management**: Play sounds from database, pull audio from URLs (YouTube, etc.)
- **User Personalization**: Custom greetings/farewells with 3-mode system (all/custom/none)
- **Command Aliases**: Create shortcuts for complex command sequences
- **Audio Effects**: Real-time audio processing with multiple effects
- **Private Messages**: Configurable private command support
- **Auto-reconnection**: Reliable session management with error recovery

## Quick Start

### Prerequisites
- Rust 1.70+
- FFmpeg (for audio processing)

### Installation

**From Releases:**
```bash
wget https://github.com/codeandkey/bigbot/releases/latest/download/bigbot-linux-x86_64.tar.gz
tar -xzf bigbot-linux-x86_64.tar.gz
chmod +x bigbot
```

**From Source:**
```bash
git clone https://github.com/codeandkey/bigbot.git
cd bigbot
cargo build --release
```

### First Run
```bash
./bigbot  # Creates ~/.bigbot/config.yml automatically
```

## Configuration

Basic configuration in `~/.bigbot/config.yml`:

```yaml
bot:
  username: "Big Bot"
  password: null

server:
  host: "localhost"
  port: 64738

behavior:
  auto_greetings: all    # all/custom/none
  auto_farewells: custom
  allow_private_commands: true
  volume: 1.0
```

## Commands

### Sound Management
```bash
!sound play [code]         # Play random or specific sound
!sound list               # Show available sounds
!sound pull <URL> <start> <length>  # Extract audio from URLs
!sound info <code>        # Sound details
```

### User Personalization
```bash
!greeting <command>       # Set join greeting
!farewell <command>       # Set leave farewell
```

### Aliases
```bash
!alias <name> <command>   # Create alias
!alias list              # Show aliases
!alias search <term>     # Search aliases
```

## Development

```bash
# Build and test
cargo build --release
cargo test

# Development with hot reload
cargo install cargo-watch
cargo watch -x 'run -- --verbose'
```

## Docker

```bash
docker build -t bigbot .
docker run -v $HOME/.bigbot:/root/.bigbot bigbot
```

## License

MIT License - see [LICENSE](LICENSE) for details.
