# threebot

[![Rust](https://github.com/codeandkey/threebot/actions/workflows/ci.yml/badge.svg)](https://github.com/codeandkey/threebot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`threebot` is a realtime Mumble bot written in Rust.  
It lets users pull short audio clips from public sources (YouTube, Instagram, Reddit, and similar links), store them, and play them back to a Mumble server with optional effects.

## What It Does

- Connects to a Mumble server and plays audio in realtime
- Extracts clips from URLs via `!sound pull <url> <start> <length>`
- Stores clips for reuse and playback by code
- Applies live effects (loud, fast, slow, phone, reverb, echo, pitch, bass, reverse, muffle)
- Supports aliases plus user greeting/farewell commands

## Quick Start

Prerequisites:

- Rust
- `ffmpeg`
- `yt-dlp`

Run from source:

```bash
git clone https://github.com/codeandkey/threebot.git
cd threebot
cargo run --release
```

First run creates config at `~/.threebot/config.yml`.

## Core Commands

```bash
!sound pull <url> <start> <length>   # Create a clip from a public source
!sound play [code] [+effects...]     # Play random/specific sound with optional effects
!sound list [page]                   # List sounds
!sound info <code>                   # Show metadata
!sound remove <code>                 # Delete sound
!alias <name> <command...>           # Create alias
!greeting <command...>               # Set join command
!farewell <command...>               # Set leave command
```

## License

MIT. See `LICENSE`.
