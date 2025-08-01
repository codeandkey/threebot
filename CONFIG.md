# Configuration

BigBot uses YAML-based configuration that auto-generates on first run.

## Location

- **Default**: `~/.bigbot/config.yml` (Linux/macOS) or `C:\Users\<username>\.bigbot\config.yml` (Windows)
- **Custom**: Use `--config /path/to/config.yml`

## Auto-Generation

Configuration is created automatically if missing:

```bash
./bigbot                              # Creates ~/.bigbot/config.yml
./bigbot --config /custom/config.yml  # Creates custom config
```

## Key Settings

```yaml
bot:
  username: "Big Bot"
  password: null
  verbose: false

server:
  host: "localhost"
  port: 64738
  timeout_seconds: 10

behavior:
  auto_greetings: all    # all/custom/none
  auto_farewells: custom
  allow_private_commands: true
  volume: 1.0

paths:
  data_dir: null         # Defaults to ~/.bigbot
```

## Command Line Overrides

```bash
./bigbot --verbose                    # Enable verbose logging
./bigbot --data-dir /custom/path      # Custom data directory
./bigbot --config /custom/config.yml # Custom config file
```

See `example-config.yml` for full documentation.