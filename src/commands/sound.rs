use super::{Command, CommandContext, SessionTools};

#[derive(Default)]
pub struct SoundCommand;

impl SoundCommand {
    /// Check if a string represents an audio effect (with or without + prefix)
    fn is_audio_effect(&self, arg: &str) -> bool {
        let effect_name = arg.strip_prefix('+').unwrap_or(arg);
        matches!(
            effect_name,
            "loud" | "fast" | "slow" | "reverb" | "echo" | "up" | "down" | "bass" | "reverse" | "muffle"
        )
    }

    /// Apply random modifiers based on behavior settings
    fn apply_random_modifiers(
        &self,
        mut effects: Vec<crate::audio::effects::AudioEffect>,
        tools: &dyn SessionTools,
    ) -> Vec<crate::audio::effects::AudioEffect> {
        let behavior = tools.behavior_settings();

        if !behavior.random_modifiers_enabled {
            return effects;
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Available effects to randomly add
        let available_effects = [
            crate::audio::effects::AudioEffect::Loud,
            crate::audio::effects::AudioEffect::Fast,
            crate::audio::effects::AudioEffect::Slow,
            crate::audio::effects::AudioEffect::Reverb,
            crate::audio::effects::AudioEffect::Echo,
            crate::audio::effects::AudioEffect::Up,
            crate::audio::effects::AudioEffect::Down,
            crate::audio::effects::AudioEffect::Bass,
            crate::audio::effects::AudioEffect::Reverse,
            crate::audio::effects::AudioEffect::Muffle,
        ];

        // Apply random modifiers for the configured number of rounds
        for round in 0..behavior.random_modifier_rounds {
            // Use system time + round as entropy for pseudo-random behavior
            let mut hasher = DefaultHasher::new();
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .hash(&mut hasher);
            round.hash(&mut hasher);

            let hash = hasher.finish();
            let random_value = (hash as f64) / (u64::MAX as f64);

            if random_value < behavior.random_modifier_chance as f64 {
                // Randomly select an effect that's not already applied
                let available: Vec<_> = available_effects
                    .iter()
                    .filter(|&effect| !effects.contains(effect))
                    .collect();

                if !available.is_empty() {
                    let index = (hash as usize) % available.len();
                    let selected_effect = available[index].clone();
                    effects.push(selected_effect);
                }
            }
        }

        effects
    }

    /// Parse a flexible timestamp format: [HH]:[MM]:<S>[.SS]
    /// Examples: "30", "1:30", "1:23:45", "1:23:45.5"
    fn parse_timestamp(input: &str) -> Result<f64, String> {
        // First try parsing as a plain number (seconds)
        if let Ok(seconds) = input.parse::<f64>() {
            return Ok(seconds);
        }

        // Split by colons to handle HH:MM:SS format
        let parts: Vec<&str> = input.split(':').collect();

        if parts.len() > 3 {
            return Err("Invalid timestamp format. Use [HH]:[MM]:<S>[.SS]".to_string());
        }

        let mut total_seconds = 0.0;

        // Parse from right to left (seconds, minutes, hours)
        for (i, part) in parts.iter().rev().enumerate() {
            let value = part
                .parse::<f64>()
                .map_err(|_| format!("Invalid number in timestamp: '{}'", part))?;

            match i {
                0 => total_seconds += value,          // seconds
                1 => total_seconds += value * 60.0,   // minutes
                2 => total_seconds += value * 3600.0, // hours
                _ => unreachable!(),
            }
        }

        Ok(total_seconds)
    }

    async fn pull_audio(
        &self,
        tools: &dyn SessionTools,
        context: &CommandContext,
        url: &str,
        start: f64,
        length: f64,
    ) -> Result<String, crate::error::Error> {
        use std::process::Command;
        use tokio::fs;

        // Get the sounds manager from session tools
        let manager = tools.get_sounds_manager().ok_or_else(|| {
            crate::error::Error::InvalidInput("Sounds manager not available".to_string())
        })?;

        // Generate a unique code for this sound
        let code = self.generate_unique_code(tools).await?;

        // Create a temporary directory for processing
        let temp_dir = std::env::temp_dir().join(format!("mumble_sound_{}", code));
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| crate::error::Error::IOError(e))?;
        // Download audio using yt-dlp
        let temp_audio_path = temp_dir.join("downloaded_audio.%(ext)s");
        let mut yt_dlp_cmd = Command::new("yt-dlp");
        yt_dlp_cmd
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0") // Best quality
            .arg("-o")
            .arg(&temp_audio_path);

        // Add cookies file if configured
        if let Some(cookies_path) = tools.external_tools_settings().get_ytdlp_cookies_path() {
            yt_dlp_cmd.arg("--cookies").arg(cookies_path);
        }

        let yt_dlp_output = yt_dlp_cmd
            .arg(url)
            .output()
            .map_err(|e| crate::error::Error::IOError(e))?;

        if !yt_dlp_output.status.success() {
            let stderr = String::from_utf8_lossy(&yt_dlp_output.stderr);
            return Err(crate::error::Error::InvalidInput(format!(
                "yt-dlp failed: {}",
                stderr
            )));
        }

        // Find the downloaded file (yt-dlp will replace %(ext)s with the actual extension)
        let mut downloaded_file = None;
        let mut entries = fs::read_dir(&temp_dir)
            .await
            .map_err(|e| crate::error::Error::IOError(e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| crate::error::Error::IOError(e))?
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("downloaded_audio.") {
                    downloaded_file = Some(path);
                    break;
                }
            }
        }

        let downloaded_path = downloaded_file.ok_or_else(|| {
            crate::error::Error::InvalidInput("Downloaded file not found".to_string())
        })?;

        // Trim the audio using ffmpeg
        let final_path = manager.sounds_dir().join(format!("{}.mp3", code));
        let ffmpeg_output = Command::new("ffmpeg")
            .arg("-i")
            .arg(&downloaded_path)
            .arg("-ss")
            .arg(start.to_string())
            .arg("-t")
            .arg(length.to_string())
            .arg("-acodec")
            .arg("mp3")
            .arg("-y") // Overwrite output file
            .arg(&final_path)
            .output()
            .map_err(|e| crate::error::Error::IOError(e))?;

        if !ffmpeg_output.status.success() {
            let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
            return Err(crate::error::Error::InvalidInput(format!(
                "ffmpeg failed: {}",
                stderr
            )));
        }

        // Clean up temp directory
        if let Err(e) = fs::remove_dir_all(&temp_dir).await {
            eprintln!("Warning: Failed to clean up temp directory: {}", e);
        }

        // Get the author name from the triggering user
        let author = if let Some(user_id) = context.triggering_user_id {
            if let Some(user_info) = tools.get_user_info(user_id) {
                user_info
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown User".to_string())
            } else {
                "Unknown User".to_string()
            }
        } else {
            "Bot".to_string()
        };

        // Add to database
        manager
            .add_sound(&code, author, Some(url.to_string()), start, length)
            .await?;

        // Automatically play the newly created sound
        if let Ok(Some(sound_file)) = manager.get_sound(&code).await {
            if sound_file.exists() {
                if let Some(file_path_str) = sound_file.path_str() {
                    let _ = tools.play_sound(file_path_str).await; // Don't fail if play fails
                }
            }
        }

        Ok(code)
    }

    async fn generate_unique_code(
        &self,
        tools: &dyn SessionTools,
    ) -> Result<String, crate::error::Error> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        // Get the sounds manager from session tools
        let manager = tools.get_sounds_manager().ok_or_else(|| {
            crate::error::Error::InvalidInput("Sounds manager not available".to_string())
        })?;

        // Try up to 100 times to generate a unique code
        for attempt in 0..100 {
            // Use system time + attempt + random component as seed for better uniqueness
            let mut hasher = DefaultHasher::new();
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .hash(&mut hasher);
            attempt.hash(&mut hasher);

            // Add more entropy by using the hasher state itself
            hasher.write_u64(hasher.finish());
            hasher.write_usize(attempt * 7919); // Use prime for better distribution

            let hash = hasher.finish();

            // Generate 4-character code from hash using larger charset
            let code: String = (0..4)
                .map(|i| {
                    let idx = ((hash >> (i * 8)) as usize) % CHARSET.len();
                    CHARSET[idx] as char
                })
                .collect();

            // Check if this code already exists
            match manager.get_sound(&code).await {
                Ok(None) => {
                    log::debug!(
                        "Generated unique sound code '{}' after {} attempts",
                        code,
                        attempt + 1
                    );
                    return Ok(code);
                }
                Ok(Some(_)) => {
                    log::debug!(
                        "Sound code '{}' already exists, trying again (attempt {})",
                        code,
                        attempt + 1
                    );
                    continue;
                }
                Err(e) => {
                    log::error!("Database error while checking sound code '{}': {}", code, e);
                    return Err(e);
                }
            }
        }

        Err(crate::error::Error::InvalidInput(
            "Failed to generate unique code after 100 attempts".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl Command for SoundCommand {
    async fn execute(
        &mut self,
        tools: &dyn SessionTools,
        _context: CommandContext,
        args: Vec<String>,
    ) -> Result<(), crate::error::Error> {
        if args.is_empty() {
            tools.reply("**🔊 Sound Command Help:**\n\
                • `!sound play` - Play a random sound (with possible random effects)\n\
                • `!sound play <code>` - Play a specific sound by code (with possible random effects)\n\
                • `!sound play <code> [effects...]` - Play a sound with audio effects\n\
                • `!sound play [+effects...]` - Play a random sound with audio effects\n\
                • `!sound list` - List all available sounds (ordered by newest first, with creation date and aliases)\n\
                • `!sound info <code>` - Show detailed information about a sound\n\
                • `!sound pull <URL> <start> <length>` - Extract audio from a video/audio URL\n\
                • `!sound scan` - Scan for orphaned sound files\n\
                • `!sound stopall` - Stop all currently playing audio streams\n\n\
                **Audio Effects:**\n\
                • `loud` - Increase volume (+6dB)\n\
                • `fast` - Increase speed/tempo (1.5x)\n\
                • `slow` - Decrease speed/tempo (0.75x)\n\
                • `reverb` - Add reverb effect\n\
                • `echo` - Add echo effect\n\
                • `up` - Pitch up (+200 cents)\n\
                • `down` - Pitch down (-200 cents)\n\
                • `bass` - Bass boost (+25dB at 50Hz)\n\
                • `reverse` - Play audio backwards\n\
                • `muffle` - Apply low-pass filter (1000Hz cutoff)\n\n\
                **Random Effects:**\n\
                • When no specific sound is provided, random effects may be applied based on server configuration\n\
                • Configure via `random_modifiers_enabled`, `random_modifier_chance`, and `random_modifier_rounds` in config.yml\n\n\
                **Pull Command Details:**\n\
                • `<URL>` - YouTube, Twitter, or other supported video/audio URL\n\
                • `<start>` - Start time (e.g., '30', '1:30', '1:23:45')\n\
                • `<length>` - Duration in seconds (e.g., '5', '10.5')\n\
                • For age-restricted or private content, configure `ytdlp_cookies_file` in config.yml\n\n\
                **Examples:**\n\
                • `!sound play` - Play random sound (may have random effects)\n\
                • `!sound play +reverb` - Play random sound with reverb\n\
                • `!sound play abc123` - Play sound with code 'abc123' (may have random effects)\n\
                • `!sound play abc123 loud fast` - Play sound with volume boost and faster tempo\n\
                • `!sound play abc123 +reverb +echo +bass` - Play sound with reverb, echo, and bass boost effects\n\
                • `!sound pull https://youtube.com/watch?v=... 1:30 5` - Extract 5 seconds starting at 1:30").await?;
            return Ok(());
        }

        match args[0].as_str() {
            "list" => {
                if let Some(manager) = tools.get_sounds_manager() {
                    match manager.list_sounds().await {
                        Ok(sounds) => {
                            if sounds.is_empty() {
                                tools.reply("📋 No sounds available").await?;
                            } else {
                                let mut response =
                                    format!("🔊 **Available Sounds** ({} total)\n\n", sounds.len());

                                // Get alias manager for looking up aliases
                                let alias_manager = tools.get_alias_manager();

                                // Prepare table data
                                let headers =
                                    &["Created", "Code", "Source", "Author", "Duration", "Aliases"];
                                let mut rows = Vec::new();

                                for sound in sounds.iter().take(30) {
                                    // Limit to first 30 to avoid message length issues
                                    let duration = format!("{:.1}s", sound.length);
                                    let source_link = if let Some(url) = &sound.source_url {
                                        format!("<a href=\"{}\">source</a>", url)
                                    } else {
                                        "-".to_string()
                                    };
                                    let author = &sound.author;
                                    let created = sound.created_at.format("%m/%d/%y").to_string();

                                    // Find aliases that use this sound
                                    let aliases_text = if let Some(alias_mgr) = &alias_manager {
                                        match alias_mgr
                                            .find_aliases_containing_sound(&sound.code)
                                            .await
                                        {
                                            Ok(aliases) => {
                                                if aliases.is_empty() {
                                                    "-".to_string()
                                                } else {
                                                    aliases
                                                        .iter()
                                                        .map(|alias| alias.name.as_str())
                                                        .collect::<Vec<_>>()
                                                        .join(", ")
                                                }
                                            }
                                            Err(_) => "?".to_string(),
                                        }
                                    } else {
                                        "?".to_string()
                                    };

                                    rows.push(vec![
                                        created,
                                        format!(
                                            "<span style=\"font-family: serif;\">{}</span>",
                                            sound.code
                                        ),
                                        source_link,
                                        author.clone(),
                                        duration,
                                        aliases_text,
                                    ]);
                                }

                                response.push_str(&tools.create_html_table(headers, &rows));

                                if sounds.len() > 50 {
                                    response.push_str(&format!(
                                        "\n\n*Showing first 30 of {} sounds*",
                                        sounds.len()
                                    ));
                                }

                                tools.reply_html(&response).await?;
                            }
                        }
                        Err(e) => {
                            tools
                                .reply(&format!("❌ Failed to list sounds: {}", e))
                                .await?;
                        }
                    }
                } else {
                    tools.reply("❌ Sounds manager not available").await?;
                }
            }
            "play" => {
                // Separate sound codes from effect modifiers
                let (sound_codes, effect_args): (Vec<_>, Vec<_>) = args
                    .iter()
                    .skip(1)
                    .partition(|arg| !self.is_audio_effect(arg));

                // Determine if we should play a random sound or a specific one
                let target_sound_code = if sound_codes.is_empty() {
                    None // Play random sound
                } else {
                    Some(sound_codes[0].clone()) // Play specific sound
                };

                // Parse effects from effect arguments
                let effect_strings: Vec<String> = effect_args
                    .into_iter()
                    .map(|s| s.strip_prefix('+').unwrap_or(s).to_string()) // Remove '+' prefix if present
                    .collect();
                let mut effects = match crate::audio::effects::parse_effects(&effect_strings) {
                    Ok(effects) => effects,
                    Err(e) => {
                        tools.reply(&format!("❌ {}", e)).await?;
                        return Ok(());
                    }
                };

                // Apply random modifiers (only for !sound play, not when specific effects are provided)
                if target_sound_code.is_none() {
                    effects = self.apply_random_modifiers(effects, tools);
                }

                if let Some(manager) = tools.get_sounds_manager() {
                    let is_random_sound = target_sound_code.is_none();
                    let (sound_file, display_code) = if let Some(code) = target_sound_code {
                        // Play specific sound
                        match manager.get_sound(&code).await {
                            Ok(Some(sound_file)) => (sound_file, code),
                            Ok(None) => {
                                tools
                                    .reply(&format!("❌ Sound '{}' not found", code))
                                    .await?;
                                return Ok(());
                            }
                            Err(e) => {
                                tools
                                    .reply(&format!("❌ Error retrieving sound '{}': {}", code, e))
                                    .await?;
                                return Ok(());
                            }
                        }
                    } else {
                        // Play random sound
                        match manager.get_random_sound().await {
                            Ok(Some(sound_file)) => {
                                let code = sound_file
                                    .metadata
                                    .as_ref()
                                    .map(|m| m.code.clone())
                                    .unwrap_or_else(|| sound_file.code.clone());
                                (sound_file, code)
                            }
                            Ok(None) => {
                                tools.reply("❌ No sounds available").await?;
                                return Ok(());
                            }
                            Err(e) => {
                                tools
                                    .reply(&format!("❌ Error getting random sound: {}", e))
                                    .await?;
                                return Ok(());
                            }
                        }
                    };

                    // Check if file exists
                    if !sound_file.exists() {
                        tools
                            .reply(&format!(
                                "❌ Sound file '{}' not found on disk",
                                display_code
                            ))
                            .await?;
                        return Ok(());
                    }

                    if let Some(file_path_str) = sound_file.path_str() {
                        let result = if effects.is_empty() {
                            tools.play_sound(file_path_str).await
                        } else {
                            tools.play_sound_with_effects(file_path_str, &effects).await
                        };

                        match result {
                            Ok(()) => {
                                let has_random_effects =
                                    effect_strings.is_empty() && !effects.is_empty();
                                let message = if !is_random_sound {
                                    // Specific sound
                                    if effects.is_empty() {
                                        format!("🔊 Playing sound '{}'", display_code)
                                    } else {
                                        let effect_names: Vec<String> = effects
                                            .iter()
                                            .map(|e| format!("{:?}", e).to_lowercase())
                                            .collect();
                                        let effect_prefix = if has_random_effects {
                                            "🎲 random "
                                        } else {
                                            ""
                                        };
                                        format!(
                                            "🔊 Playing sound '{}' with {}effects: {}",
                                            display_code,
                                            effect_prefix,
                                            effect_names.join(", ")
                                        )
                                    }
                                } else {
                                    // Random sound
                                    if effects.is_empty() {
                                        format!("🎲 Playing random sound '{}'", display_code)
                                    } else {
                                        let effect_names: Vec<String> = effects
                                            .iter()
                                            .map(|e| format!("{:?}", e).to_lowercase())
                                            .collect();
                                        let effect_prefix = if has_random_effects {
                                            "🎲 random "
                                        } else {
                                            ""
                                        };
                                        format!(
                                            "🎲 Playing random sound '{}' with {}effects: {}",
                                            display_code,
                                            effect_prefix,
                                            effect_names.join(", ")
                                        )
                                    }
                                };
                                tools.reply(&message).await?;
                            }
                            Err(e) => {
                                tools
                                    .reply(&format!(
                                        "❌ Failed to play sound '{}': {}",
                                        display_code, e
                                    ))
                                    .await?;
                            }
                        }
                    } else {
                        tools
                            .reply(&format!(
                                "❌ Invalid file path for sound '{}'",
                                display_code
                            ))
                            .await?;
                    }
                } else {
                    tools.reply("❌ Sounds manager not available").await?;
                }
            }
            "info" => {
                if args.len() < 2 {
                    tools.reply("Usage: !sound info <code>").await?;
                } else {
                    let code = &args[1];
                    if let Some(manager) = tools.get_sounds_manager() {
                        match manager.get_sound(code).await {
                            Ok(Some(sound_file)) => {
                                let mut response =
                                    format!("🔊 **Sound Information: {}**\n\n", code);

                                if let Some(metadata) = &sound_file.metadata {
                                    response
                                        .push_str(&format!("**Author:** {}\n", metadata.author));
                                    response.push_str(&format!(
                                        "**Duration:** {:.1} seconds\n",
                                        metadata.length
                                    ));
                                    response.push_str(&format!(
                                        "**Start Time:** {}\n",
                                        metadata.start_time
                                    ));

                                    if let Some(source_url) = &metadata.source_url {
                                        response.push_str(&format!("**Source:** {}\n", source_url));
                                    }

                                    response.push_str(&format!(
                                        "**Created:** {}\n",
                                        metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                                    ));
                                }

                                // File information
                                if let Some(path) = sound_file.file_path.to_str() {
                                    response.push_str(&format!("**File Path:** `{}`\n", path));
                                }

                                // Check if file exists
                                if sound_file.exists() {
                                    response.push_str("**Status:** ✅ File exists on disk\n");

                                    // Get file size if possible
                                    if let Ok(metadata) = std::fs::metadata(&sound_file.file_path) {
                                        let size_kb = metadata.len() as f64 / 1024.0;
                                        if size_kb < 1024.0 {
                                            response.push_str(&format!(
                                                "**File Size:** {:.1} KB\n",
                                                size_kb
                                            ));
                                        } else {
                                            response.push_str(&format!(
                                                "**File Size:** {:.1} MB\n",
                                                size_kb / 1024.0
                                            ));
                                        }
                                    }
                                } else {
                                    response.push_str("**Status:** ❌ File missing from disk\n");
                                }

                                tools.reply(&response).await?;
                            }
                            Ok(None) => {
                                tools
                                    .reply(&format!("❌ Sound '{}' not found", code))
                                    .await?;
                            }
                            Err(e) => {
                                tools
                                    .reply(&format!("❌ Error retrieving sound info: {}", e))
                                    .await?;
                            }
                        }
                    } else {
                        tools.reply("❌ Sounds manager not available").await?;
                    }
                }
            }
            "pull" => {
                if args.len() < 4 {
                    tools.reply("Usage: !sound pull <URL> <start> <length_seconds>\nStart format: seconds (e.g., '30'), MM:SS (e.g., '1:30'), or HH:MM:SS (e.g., '1:23:45'), optionally with subsecond precision").await?;
                } else {
                    let url = &args[1];
                    let start_str = &args[2];
                    let length_str = &args[3];

                    // Parse start and length
                    let start = match Self::parse_timestamp(start_str) {
                        Ok(s) => s,
                        Err(err) => {
                            tools.reply(&format!("Error: {}", err)).await?;
                            return Ok(());
                        }
                    };

                    let length = match length_str.parse::<f64>() {
                        Ok(l) => l,
                        Err(_) => {
                            tools
                                .reply("Error: length_seconds must be a valid number")
                                .await?;
                            return Ok(());
                        }
                    };

                    if let Some(_manager) = tools.get_sounds_manager() {
                        match self.pull_audio(tools, &_context, url, start, length).await {
                            Ok(code) => {
                                tools
                                    .reply(&format!(
                                        "✅ Successfully pulled audio and saved as sound '{}' 🔊",
                                        code
                                    ))
                                    .await?;
                            }
                            Err(e) => {
                                tools
                                    .reply(&format!("❌ Error pulling audio: {}", e))
                                    .await?;
                            }
                        }
                    } else {
                        tools.reply("Sounds manager not available").await?;
                    }
                }
            }
            "scan" => {
                if let Some(manager) = tools.get_sounds_manager() {
                    match manager.scan_orphaned_files().await {
                        Ok(orphaned_files) => {
                            if orphaned_files.is_empty() {
                                tools.reply("✅ No orphaned sound files found - all files are properly registered in the database").await?;
                            } else {
                                let mut response = format!(
                                    "🔍 **Orphaned Sound Files Found** ({} files)\n\n",
                                    orphaned_files.len()
                                );
                                response.push_str("The following files exist on disk but are not registered in the database:\n\n");

                                for (i, file) in orphaned_files.iter().enumerate() {
                                    if i < 20 {
                                        // Limit to first 20 to avoid message length issues
                                        response.push_str(&format!("• `{}`\n", file));
                                    }
                                }

                                if orphaned_files.len() > 20 {
                                    response.push_str(&format!(
                                        "\n*... and {} more files*\n",
                                        orphaned_files.len() - 20
                                    ));
                                }

                                response.push_str("\nThese files can be safely deleted or re-registered in the database.");
                                tools.reply(&response).await?;
                            }
                        }
                        Err(e) => {
                            tools
                                .reply(&format!("❌ Error scanning for orphaned files: {}", e))
                                .await?;
                        }
                    }
                } else {
                    tools.reply("❌ Sounds manager not available").await?;
                }
            }
            "stopall" => {
                tools.stop_all_streams().await?;
                tools.reply("🛑 Stopped all audio streams").await?;
            }
            _ => {
                tools.reply("❌ Unknown command. Use `!sound` (without arguments) to see available commands.").await?;
            }
        }

        Ok(())
    }

    fn description(&self) -> &str {
        "Manage and play sound files - play, list, get info, pull from URLs, and scan for orphaned files"
    }
}

#[cfg(test)]
mod tests {
    use super::SoundCommand;

    #[test]
    fn test_timestamp_parsing() {
        // Test plain seconds
        assert_eq!(SoundCommand::parse_timestamp("30").unwrap(), 30.0);
        assert_eq!(SoundCommand::parse_timestamp("45.5").unwrap(), 45.5);

        // Test MM:SS format
        assert_eq!(SoundCommand::parse_timestamp("1:30").unwrap(), 90.0);
        assert_eq!(SoundCommand::parse_timestamp("2:15").unwrap(), 135.0);
        assert_eq!(SoundCommand::parse_timestamp("0:45.5").unwrap(), 45.5);

        // Test HH:MM:SS format
        assert_eq!(SoundCommand::parse_timestamp("1:23:45").unwrap(), 5025.0);
        assert_eq!(SoundCommand::parse_timestamp("0:1:30").unwrap(), 90.0);
        assert_eq!(SoundCommand::parse_timestamp("2:0:0").unwrap(), 7200.0);
        assert_eq!(SoundCommand::parse_timestamp("1:23:45.5").unwrap(), 5025.5);

        // Test error cases
        assert!(SoundCommand::parse_timestamp("invalid").is_err());
        assert!(SoundCommand::parse_timestamp("1:2:3:4").is_err());
        assert!(SoundCommand::parse_timestamp("1:invalid:30").is_err());
    }
}
