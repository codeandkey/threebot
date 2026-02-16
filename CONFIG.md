# Configuration

THREEBOT uses YAML-based configuration that auto-generates on first run.

## Location

- **Default**: `~/.THREEBOT/config.yml` (Linux/macOS) or `C:\Users\<username>\.THREEBOT\config.yml` (Windows)
- **Custom**: Use `--config /path/to/config.yml`

## Auto-Generation

Configuration is created automatically if missing:

```bash
./THREEBOT                              # Creates ~/.THREEBOT/config.yml
./THREEBOT --config /custom/config.yml  # Creates custom config
```

## Key Settings

```yaml
bot:
  username: "Threebot"
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
  data_dir: null         # Defaults to ~/.THREEBOT
```

## Command Line Overrides

```bash
./THREEBOT --verbose                    # Enable verbose logging
./THREEBOT --data-dir /custom/path      # Custom data directory
./THREEBOT --config /custom/config.yml # Custom config file
```

See `example-config.yml` for full documentation.
