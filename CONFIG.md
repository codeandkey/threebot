# Configuration System

Big Bot uses a YAML-based configuration system that allows you to customize all aspects of the bot's behavior without recompiling.

## Configuration File Location

By default, the configuration file is located at:
- **Linux/macOS**: `~/.bigbot/config.yml`
- **Windows**: `C:\Users\<username>\.bigbot\config.yml`

You can specify a custom configuration file path using the `--config` option.

## Generating Default Configuration

The bot automatically creates a fully documented configuration file when it runs for the first time. If no configuration file is found at the expected location, the bot will:

1. Create the configuration directory (`~/.bigbot`) if it doesn't exist
2. Generate a complete example configuration file with all settings documented
3. Use the generated configuration to start the bot

You can also specify a custom configuration file location using the `--config` option. If the specified file doesn't exist, it will be created automatically with the example configuration.

**Example:**
```bash
# This will auto-create ~/.bigbot/config.yml if it doesn't exist
./bigbot

# This will auto-create /path/to/custom/config.yml if it doesn't exist  
./bigbot --config /path/to/custom/config.yml
```

## Configuration Sections

### Bot Settings (`bot`)
- `username`: The display name for the bot on the Mumble server
- `password`: Optional password for server authentication (use `null` if not needed)
- `verbose`: Enable detailed logging output

### Server Settings (`server`)
- `host`: Hostname or IP address of the Mumble server
- `port`: Port number (default: 64738)
- `timeout_seconds`: Connection timeout in seconds

Several more sections are documented in the example config.

## Example Configuration

See `example-config.yml` in the project root for a fully documented example configuration.

## Command Line Overrides

Several configuration options can be overridden from the command line:

```bash
# Enable verbose logging (overrides config file)
./bigbot --verbose

# Use custom data directory (overrides config file)
./bigbot --data-dir /path/to/custom/data

# Use custom configuration file
./bigbot --config /path/to/custom/config.yml
```