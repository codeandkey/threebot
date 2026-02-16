use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot configuration
    pub bot: BotSettings,
    /// Server connection settings
    pub server: ServerSettings,
    /// Audio and behavior settings
    pub behavior: BehaviorSettings,
    /// Audio effect parameters
    pub audio_effects: AudioEffectSettings,
    /// Paths and directories
    pub paths: PathSettings,
    /// External tools configuration
    pub external_tools: ExternalToolsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSettings {
    /// Username for the bot
    pub username: String,
    /// Optional password for authentication
    pub password: Option<String>,
    /// Whether to enable verbose logging
    pub verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Server hostname or IP address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    /// Greeting behavior when users join
    pub auto_greetings: GreetingMode,
    /// Farewell behavior when users leave
    pub auto_farewells: FarewellMode,
    /// Whether to respond to commands in private messages
    pub allow_private_commands: bool,
    /// Global volume multiplier for all outgoing audio (1.0 = normal, 0.5 = half volume, 2.0 = double volume)
    pub volume: f32,
    /// Enable volume normalization to maintain consistent loudness levels
    pub volume_normalization_enabled: bool,
    /// Target loudness level for normalization (in LUFS, typically -23 to -16)
    pub target_loudness_lufs: f32,
    /// Maximum gain boost allowed during normalization (in dB, prevents over-amplification)
    pub max_normalization_gain_db: f32,
    /// Enable random modifiers when playing sounds
    pub random_modifiers_enabled: bool,
    /// Probability (0.0-1.0) for each round of random modifier application
    pub random_modifier_chance: f32,
    /// Number of rounds to potentially apply random modifiers
    pub random_modifier_rounds: u32,
    /// Audio buffer size in samples (larger = more latency but smoother on slow machines)
    pub audio_buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEffectSettings {
    /// Volume boost for 'loud' effect (in dB)
    pub loud_boost_db: f32,
    /// Speed multiplier for 'fast' effect
    pub fast_speed_multiplier: f32,
    /// Speed multiplier for 'slow' effect
    pub slow_speed_multiplier: f32,
    /// Pitch shift for 'up' effect (in cents, 100 cents = 1 semitone)
    pub pitch_up_cents: i32,
    /// Pitch shift for 'down' effect (in cents, negative values lower pitch)
    pub pitch_down_cents: i32,
    /// Bass boost frequency for 'bass' effect (in Hz)
    pub bass_boost_frequency_hz: f32,
    /// Bass boost gain for 'bass' effect (in dB)
    pub bass_boost_gain_db: f32,
    /// Reverb room size (0.0-1.0, larger = more reverb)
    pub reverb_room_size: f32,
    /// Reverb damping (0.0-1.0, higher = less bright reverb)
    pub reverb_damping: f32,
    /// Echo delay time (in milliseconds)
    pub echo_delay_ms: u32,
    /// Echo feedback amount (0.0-1.0, higher = more repeats)
    pub echo_feedback: f32,
    /// Low-pass filter cutoff frequency for 'muffle' effect (in Hz)
    pub muffle_cutoff_frequency_hz: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GreetingMode {
    /// Play both custom greetings and random sounds (custom preferred, fallback to random)
    All,
    /// Only play user-specified custom greetings (silent if none set)
    Custom,
    /// No automatic greeting sounds
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FarewellMode {
    /// Play both custom farewells and random sounds (custom preferred, fallback to random)
    All,
    /// Only play user-specified custom farewells (silent if none set)
    Custom,
    /// No automatic farewell sounds
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSettings {
    /// Directory to store bot data (sounds, database, etc.)
    pub data_dir: Option<String>,
    /// Path to SSL certificate file
    pub cert_file: Option<String>,
    /// Path to SSL private key file
    pub key_file: Option<String>,
    /// Directory containing trusted certificates for server verification
    pub trusted_certs_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolsSettings {
    /// Path to cookies file for yt-dlp (for authentication and age-restricted content)
    pub ytdlp_cookies_file: Option<String>,
}

impl ExternalToolsSettings {
    /// Get the expanded path for the yt-dlp cookies file, handling tilde expansion
    pub fn get_ytdlp_cookies_path(&self) -> Option<PathBuf> {
        self.ytdlp_cookies_file.as_ref().map(|path| {
            if path.starts_with("~/") {
                // Expand tilde to home directory
                if let Some(home_dir) = dirs::home_dir() {
                    home_dir.join(&path[2..]) // Skip the "~/" part
                } else {
                    // Fallback if home directory can't be determined
                    PathBuf::from(path)
                }
            } else if path == "~" {
                // Handle case where path is just "~"
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            } else {
                // Path doesn't start with tilde, use as-is
                PathBuf::from(path)
            }
        })
    }
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            bot: BotSettings {
                username: "Threebot".to_string(),
                password: None,
                verbose: false,
            },
            server: ServerSettings {
                host: "localhost".to_string(),
                port: 64738,
                timeout_seconds: 10,
            },
            behavior: BehaviorSettings {
                auto_greetings: GreetingMode::All,
                auto_farewells: FarewellMode::Custom,
                allow_private_commands: true,
                volume: 1.0,
                volume_normalization_enabled: false,
                target_loudness_lufs: -18.0, // Good balance for voice chat
                max_normalization_gain_db: 12.0, // Prevent excessive amplification
                random_modifiers_enabled: true,
                random_modifier_chance: 0.05, // 5% chance per round
                random_modifier_rounds: 2,
                audio_buffer_size: 8192, // Default buffer size (good balance of latency vs performance)
            },
            audio_effects: AudioEffectSettings {
                loud_boost_db: 6.0,
                fast_speed_multiplier: 1.5,
                slow_speed_multiplier: 0.75,
                pitch_up_cents: 200,
                pitch_down_cents: -200,
                bass_boost_frequency_hz: 50.0,
                bass_boost_gain_db: 25.0,
                reverb_room_size: 1.0, // Was 0.5, now 1.0 to match "100" parameter
                reverb_damping: 1.0,   // Was 0.5, now 1.0 to match "100" parameter
                echo_delay_ms: 300,
                echo_feedback: 0.3,
                muffle_cutoff_frequency_hz: 1000.0, // Default cutoff frequency for low-pass filter
            },
            paths: PathSettings {
                data_dir: None,
                cert_file: None,
                key_file: None,
                trusted_certs_dir: None,
            },
            external_tools: ExternalToolsSettings {
                ytdlp_cookies_file: None,
            },
        }
    }
}

impl BotConfig {
    /// Load configuration from a YAML file, creating default if it doesn't exist
    pub fn load_or_create<P: AsRef<std::path::Path>>(config_path: P) -> Result<Self, Error> {
        let config_path = config_path.as_ref();

        if config_path.exists() {
            // Load existing configuration
            let config_content = std::fs::read_to_string(config_path)
                .map_err(|e| Error::ConfigError(format!("Failed to read config file: {}", e)))?;

            let config: BotConfig = serde_yaml::from_str(&config_content)
                .map_err(|e| Error::ConfigError(format!("Failed to parse config file: {}", e)))?;

            info!("Loaded configuration from {}", config_path.display());
            Ok(config)
        } else {
            // Create configuration from example config
            let example_config = Self::get_example_config_content();
            Self::create_config_from_content(config_path, &example_config)?;

            // Parse the example config to return a BotConfig instance
            let config: BotConfig = serde_yaml::from_str(&example_config).map_err(|e| {
                Error::ConfigError(format!("Failed to parse example config: {}", e))
            })?;

            info!(
                "Created configuration from example at {}",
                config_path.display()
            );
            Ok(config)
        }
    }

    /// Get the content of the example configuration
    fn get_example_config_content() -> String {
        r#"# Threebot Configuration File
# This file contains all the settings for the Threebot Mumble bot.
# You can override most of these settings using command-line arguments.

# Bot-specific settings
bot:
  # The username the bot will use when connecting to the server
  username: "Threebot"
  # Optional password for server authentication (leave as null if not needed)
  password: null
  # Enable verbose logging (can be overridden with --verbose)
  verbose: false

# Mumble server connection settings  
server:
  # Hostname or IP address of the Mumble server
  host: "localhost"
  # Port number for the Mumble server (default: 64738)
  port: 64738
  # Connection timeout in seconds
  timeout_seconds: 10

# Bot behavior settings
behavior:
  # Greeting sounds when users join
  # Options: "all" (custom + random fallback), "custom" (only user-set), "none" (silent)
  auto_greetings: all
  # Farewell sounds when users leave  
  # Options: "all" (custom + random fallback), "custom" (only user-set), "none" (silent)
  auto_farewells: custom
  # Allow users to send commands via private messages
  allow_private_commands: true
  # Global volume multiplier for all outgoing audio (1.0 = normal, 0.5 = half volume, 2.0 = double volume)
  volume: 1.0
  # Enable volume normalization to maintain consistent loudness levels
  volume_normalization_enabled: false
  # Target loudness level for normalization (in LUFS, typically -23 to -16)
  target_loudness_lufs: -18.0
  # Maximum gain boost allowed during normalization (in dB, prevents over-amplification)
  max_normalization_gain_db: 12.0
  # Enable random audio effects when playing sounds
  random_modifiers_enabled: true
  # Probability (0.0-1.0) for each round of random modifier application (0.05 = 5% chance)
  random_modifier_chance: 0.05
  # Number of rounds to potentially apply random modifiers (2 = two 5% chances)
  random_modifier_rounds: 2
  # Audio buffer size in bytes (larger = more latency but smoother on slow machines)
  # Default: 8192, Low-end machines: 16384 or 32768, High-end machines: 4096
  audio_buffer_size: 8192

# Audio effect parameters
audio_effects:
  # Volume boost for 'loud' effect (in dB)
  loud_boost_db: 6.0
  # Speed multiplier for 'fast' effect
  fast_speed_multiplier: 1.5
  # Speed multiplier for 'slow' effect  
  slow_speed_multiplier: 0.75
  # Pitch shift for 'up' effect (in cents, 100 cents = 1 semitone)
  pitch_up_cents: 200
  # Pitch shift for 'down' effect (in cents, negative values lower pitch)
  pitch_down_cents: -200
  # Bass boost frequency for 'bass' effect (in Hz)
  bass_boost_frequency_hz: 50.0
  # Bass boost gain for 'bass' effect (in dB)
  bass_boost_gain_db: 25.0
  # Reverb room size (0.0-1.0, larger = more reverb)
  reverb_room_size: 1.0
  # Reverb damping (0.0-1.0, higher = less bright reverb)
  reverb_damping: 1.0
  # Echo delay time (in milliseconds)
  echo_delay_ms: 300
  # Echo feedback amount (0.0-1.0, higher = more repeats)
  echo_feedback: 0.3
  # Low-pass filter cutoff frequency for 'muffle' effect (in Hz)
  muffle_cutoff_frequency_hz: 1000

# File and directory paths
paths:
  # Directory to store bot data (sounds, database, certificates, etc.)
  # If null, defaults to ~/.threebot
  data_dir: null
  # Path to SSL certificate file (if null, uses data_dir/cert.pem)
  cert_file: null
  # Path to SSL private key file (if null, uses data_dir/key.pem)  
  key_file: null
  # Directory for trusted server certificates (if null, uses data_dir/trusted_certificates)
  trusted_certs_dir: null

# External tools configuration
external_tools:
  # Path to cookies file for yt-dlp (for authentication and age-restricted content)
  # Example: "/path/to/cookies.txt" or "~/.config/yt-dlp/cookies.txt"
  ytdlp_cookies_file: null
"#.to_string()
    }

    /// Create a configuration file from given content
    fn create_config_from_content<P: AsRef<std::path::Path>>(
        config_path: P,
        content: &str,
    ) -> Result<(), Error> {
        let config_path = config_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        std::fs::write(config_path, content)
            .map_err(|e| Error::ConfigError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Save configuration to a YAML file
    pub fn save<P: AsRef<std::path::Path>>(&self, config_path: P) -> Result<(), Error> {
        let config_path = config_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        let config_content = serde_yaml::to_string(self)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(config_path, config_content)
            .map_err(|e| Error::ConfigError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get the data directory path, using default if not specified
    pub fn get_data_dir(&self) -> PathBuf {
        if let Some(data_dir) = &self.paths.data_dir {
            PathBuf::from(data_dir)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".threebot")
        }
    }

    /// Get the certificate file path, using default if not specified
    pub fn get_cert_path(&self) -> PathBuf {
        if let Some(cert_file) = &self.paths.cert_file {
            PathBuf::from(cert_file)
        } else {
            self.get_data_dir().join("cert.pem")
        }
    }

    /// Get the private key file path, using default if not specified
    pub fn get_key_path(&self) -> PathBuf {
        if let Some(key_file) = &self.paths.key_file {
            PathBuf::from(key_file)
        } else {
            self.get_data_dir().join("key.pem")
        }
    }

    /// Get the trusted certificates directory path, using default if not specified
    pub fn get_trusted_certs_dir(&self) -> PathBuf {
        if let Some(trusted_certs_dir) = &self.paths.trusted_certs_dir {
            PathBuf::from(trusted_certs_dir)
        } else {
            self.get_data_dir().join("trusted_certificates")
        }
    }

    /// Get the configuration file path for the bot
    pub fn get_config_path() -> PathBuf {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home_dir.join(".threebot").join("config.yml")
    }

    /// Merge command-line overrides into the configuration
    pub fn apply_cli_overrides(&mut self, verbose: Option<bool>, data_dir: Option<String>) {
        if let Some(verbose) = verbose {
            self.bot.verbose = verbose;
        }

        if let Some(data_dir) = data_dir {
            self.paths.data_dir = Some(data_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BotConfig::default();
        assert_eq!(config.bot.username, "Threebot");
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 64738);
        assert!(matches!(config.behavior.auto_greetings, GreetingMode::All));
        assert!(matches!(
            config.behavior.auto_farewells,
            FarewellMode::Custom
        ));
    }

    #[test]
    fn test_config_serialization() {
        let config = BotConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: BotConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.bot.username, parsed.bot.username);
        assert_eq!(config.server.host, parsed.server.host);
        assert_eq!(config.server.port, parsed.server.port);
    }

    #[test]
    fn test_path_resolution() {
        let config = BotConfig::default();
        let data_dir = config.get_data_dir();
        let cert_path = config.get_cert_path();
        let key_path = config.get_key_path();

        assert!(cert_path.ends_with("cert.pem"));
        assert!(key_path.ends_with("key.pem"));
        assert!(cert_path.starts_with(&data_dir));
        assert!(key_path.starts_with(&data_dir));
    }

    #[test]
    fn test_cli_overrides() {
        let mut config = BotConfig::default();
        config.apply_cli_overrides(Some(true), Some("/custom/path".to_string()));

        assert!(config.bot.verbose);
        assert_eq!(config.paths.data_dir, Some("/custom/path".to_string()));
    }

    #[test]
    fn test_tilde_expansion() {
        let mut external_tools = ExternalToolsSettings {
            ytdlp_cookies_file: Some("~/cookies.txt".to_string()),
        };

        let expanded_path = external_tools.get_ytdlp_cookies_path().unwrap();
        assert!(expanded_path.ends_with("cookies.txt"));
        assert!(!expanded_path.to_string_lossy().starts_with("~"));

        // Test absolute path (no expansion)
        external_tools.ytdlp_cookies_file = Some("/absolute/path/cookies.txt".to_string());
        let absolute_path = external_tools.get_ytdlp_cookies_path().unwrap();
        assert_eq!(absolute_path, PathBuf::from("/absolute/path/cookies.txt"));

        // Test just tilde
        external_tools.ytdlp_cookies_file = Some("~".to_string());
        let home_path = external_tools.get_ytdlp_cookies_path().unwrap();
        if let Some(expected_home) = dirs::home_dir() {
            assert_eq!(home_path, expected_home);
        }

        // Test None
        external_tools.ytdlp_cookies_file = None;
        assert!(external_tools.get_ytdlp_cookies_path().is_none());
    }
}
