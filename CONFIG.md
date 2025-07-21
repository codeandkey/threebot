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

### Behavior Settings (`behavior`)
- `auto_greetings`: Play greeting sounds when users join (default: true)
  - When `true`: Plays custom greeting if set, otherwise plays random sound
  - When `false`: No automatic greeting sounds are played
- `auto_farewells`: Play farewell sounds when users leave (default: false)
  - When `true`: Plays custom farewell if set, stays silent otherwise  
  - When `false`: No automatic farewell sounds are played
- `allow_private_commands`: Allow commands via private messages (default: true)
  - When `true`: Users can send commands via private messages
  - When `false`: Private commands are rejected with an error message

### Path Settings (`paths`)
All paths are optional and will use sensible defaults if set to `null`:
- `data_dir`: Directory for bot data (default: `~/.bigbot`)
- `cert_file`: SSL certificate file (default: `<data_dir>/cert.pem`)
- `key_file`: SSL private key file (default: `<data_dir>/key.pem`) 
- `trusted_certs_dir`: Trusted certificates directory (default: `<data_dir>/trusted_certificates`)

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

## Example Configuration

See `example-config.yml` in the project root for a fully documented example configuration.

## Migration from Previous Versions

If you were using Big Bot before the configuration system was introduced, your existing data directory and certificates will continue to work. The bot will automatically generate a fully documented configuration file on first run with the new version.

The generated configuration will use sensible defaults that match the previous behavior:
- Username: "Big Bot" (can be changed in config)
- Server: "localhost:64738" (change to your server)
- Auto greetings: enabled
- Auto farewells: disabled  
- Private commands: enabled

## Behavior Settings Details

The behavior settings provide fine-grained control over the bot's automatic actions:

### Auto Greetings (`auto_greetings`)
Controls whether the bot automatically plays sounds when users join:
- **Enabled (true)**: When a user joins, the bot will:
  1. Check if the user has a custom greeting set via `!greeting set <command>`
  2. If yes, execute that command (e.g., `!sounds play welcome`)
  3. If no custom greeting, play a random sound via `!sounds play`
  4. If the custom greeting command fails, fall back to random sound
- **Disabled (false)**: No automatic sounds when users join

### Auto Farewells (`auto_farewells`) 
Controls whether the bot automatically plays sounds when users leave:
- **Enabled (true)**: When a user leaves, the bot will:
  1. Check if the user has a custom farewell set via `!farewell set <command>`
  2. If yes, execute that command (e.g., `!sounds play goodbye`)
  3. If no custom farewell, stay silent (no random sound)
  4. If the custom farewell command fails, stay silent
- **Disabled (false)**: No automatic sounds when users leave

### Private Commands (`allow_private_commands`)
Controls whether users can send commands via private messages:
- **Enabled (true)**: Users can send commands like `!sounds play hello` via private message
- **Disabled (false)**: Private commands are rejected with the message "Private commands are disabled on this bot."

These settings allow you to customize the bot's behavior for different server environments. For example:
- Quiet servers might disable auto greetings/farewells
- Public servers might disable private commands to encourage channel interaction
- Party servers might enable all automatic features for maximum engagement
