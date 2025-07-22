use std::process::Stdio;
use std::path::Path;
use crate::error::Error;

/// Available audio effects that can be applied to sounds
#[derive(Debug, Clone, PartialEq)]
pub enum AudioEffect {
    Loud,      // Increase volume
    Fast,      // Increase speed/tempo
    Slow,      // Decrease speed/tempo  
    Reverb,    // Add reverb effect
    Echo,      // Add echo effect
    Up,        // Pitch up
    Down,      // Pitch down
    Bass,      // Bass boost
}

impl AudioEffect {
    /// Parse a string into an AudioEffect
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "loud" => Some(AudioEffect::Loud),
            "fast" => Some(AudioEffect::Fast),
            "slow" => Some(AudioEffect::Slow),
            "reverb" => Some(AudioEffect::Reverb),
            "echo" => Some(AudioEffect::Echo),
            "up" => Some(AudioEffect::Up),
            "down" => Some(AudioEffect::Down),
            "bass" => Some(AudioEffect::Bass),
            _ => None,
        }
    }

    /// Get a description of the effect
    pub fn description(&self) -> &'static str {
        match self {
            AudioEffect::Loud => "Increase volume (+6dB)",
            AudioEffect::Fast => "Increase speed/tempo (1.5x)",
            AudioEffect::Slow => "Decrease speed/tempo (0.75x)",
            AudioEffect::Reverb => "Add reverb effect",
            AudioEffect::Echo => "Add echo effect",
            AudioEffect::Up => "Pitch up (+200 cents)",
            AudioEffect::Down => "Pitch down (-200 cents)",
            AudioEffect::Bass => "Bass boost (+25dB at 50Hz)",
        }
    }

    /// Get the ffmpeg filter string for this effect
    fn to_ffmpeg_filter(&self) -> &'static str {
        match self {
            AudioEffect::Loud => "volume=6dB",
            AudioEffect::Fast => "atempo=1.5",
            AudioEffect::Slow => "atempo=0.75",
            AudioEffect::Reverb => panic!("Reverb effect should be handled by sox, not ffmpeg"),
            AudioEffect::Echo => "aecho=0.8:0.9:1000:0.3",
            AudioEffect::Up => "asetrate=48000*1.122462,aresample=48000",    // +200 cents
            AudioEffect::Down => "asetrate=48000*0.890899,aresample=48000",  // -200 cents
            AudioEffect::Bass => "equalizer=f=50:width_type=h:width=50:g=25", // +25dB bass boost at 50Hz
        }
    }

    /// Check if this effect requires sox processing
    fn requires_sox(&self) -> bool {
        matches!(self, AudioEffect::Reverb)
    }
}

/// Represents a single stage in the audio processing pipeline
enum PipelineStage {
    Ffmpeg {
        command: tokio::process::Command,
    },
    Sox {
        command: tokio::process::Command,
    },
}

/// Builder for creating composable audio processing pipelines
/// 
/// This system allows building flexible pipelines by composing individual stages:
/// - Ffmpeg stages for format conversion and most audio effects
/// - Sox stages for reverb processing that requires sox
/// - Common async piping code that connects stages together
/// 
/// Examples:
/// - No effects: ffmpeg (format conversion only)
/// - Ffmpeg effects only: ffmpeg -> ffmpeg (with filters)
/// - Reverb only: ffmpeg -> sox
/// - Mixed effects: ffmpeg -> sox -> ffmpeg (format + reverb + other effects)
struct PipelineBuilder {
    stages: Vec<PipelineStage>,
}

impl PipelineBuilder {
    fn new() -> Self {
        Self {
            stages: Vec::new(),
        }
    }
    
    /// Add an ffmpeg stage (typically used for initial file processing or final output)
    fn add_ffmpeg_stage(&mut self, mut command: tokio::process::Command, filter_chain: Option<String>, output_format: &str) -> Result<(), Error> {
        // Configure the ffmpeg command for piping
        if let Some(filters) = &filter_chain {
            command.arg("-af").arg(filters);
        }
        
        // For final PCM output, add codec and sample rate configuration BEFORE format
        if output_format == "s16le" {
            command
                .arg("-acodec").arg("pcm_s16le")
                .arg("-ar").arg("48000")
                .arg("-ac").arg("2");
        }
        
        command
            .arg("-f").arg(output_format) // Output format (wav for intermediate, s16le for final)
            .arg("-")                     // Output to stdout
            .arg("-y")                    // Overwrite without asking
            .stdin(Stdio::null())         // No input for first stage
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());      // Capture stderr for debugging
            
        self.stages.push(PipelineStage::Ffmpeg { command });
        Ok(())
    }
    
    /// Add an ffmpeg stage that reads PCM from the previous stage via pipe
    fn add_ffmpeg_stage_with_input_pipe(&mut self, filter_chain: Option<String>) -> Result<(), Error> {
        let mut command = tokio::process::Command::new("ffmpeg");
        command
            .arg("-f").arg("s16le")       // Input format: PCM s16le
            .arg("-ar").arg("48000")      // Input sample rate: 48000 Hz
            .arg("-ac").arg("2")          // Input channels: 2 (stereo)
            .arg("-i").arg("pipe:0");     // Read from stdin
            
        if let Some(filters) = &filter_chain {
            command.arg("-af").arg(filters);
        }
        
        command
            .arg("-acodec").arg("pcm_s16le") // Output codec: PCM s16le
            .arg("-ar").arg("48000")         // Output sample rate: 48000 Hz
            .arg("-ac").arg("2")             // Output channels: 2 (stereo)
            .arg("-f").arg("s16le")          // Output format: PCM s16le
            .arg("-")                        // Output to stdout
            .arg("-y")                       // Overwrite without asking
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());         // Capture stderr for debugging
            
        self.stages.push(PipelineStage::Ffmpeg { command });
        Ok(())
    }
    
    /// Add a sox stage for reverb processing with PCM input/output
    fn add_sox_stage(&mut self) -> Result<(), Error> {
        let mut command = tokio::process::Command::new("sox");
        command
            .arg("-t").arg("raw")         // Input type: raw PCM
            .arg("-r").arg("48000")       // Sample rate: 48000 Hz
            .arg("-e").arg("signed-integer") // Encoding: signed integer
            .arg("-b").arg("16")          // Bit depth: 16 bits
            .arg("-c").arg("2")           // Channels: 2 (stereo)
            .arg("-")                     // Read from stdin
            .arg("-t").arg("raw")         // Output type: raw PCM
            .arg("-r").arg("48000")       // Sample rate: 48000 Hz
            .arg("-e").arg("signed-integer") // Encoding: signed integer
            .arg("-b").arg("16")          // Bit depth: 16 bits
            .arg("-c").arg("2")           // Channels: 2 (stereo)
            .arg("-")                     // Output to stdout
            .args(["gain", "-3", "pad", "0", "4", "reverb", "100", "100", "100", "100", "200"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());      // Capture stderr for debugging
            
        self.stages.push(PipelineStage::Sox { command });
        Ok(())
    }
    
    /// Execute the complete pipeline with async streaming, returns the final process for streaming
    async fn execute_streaming(self) -> Result<tokio::process::Child, Error> {
        if self.stages.is_empty() {
            return Err(Error::InvalidInput("No pipeline stages configured".to_string()));
        }
        
        log::info!("Starting pipeline execution with {} stages", self.stages.len());
        
        // Start all processes
        let mut processes: Vec<tokio::process::Child> = Vec::new();
        
        for (i, stage) in self.stages.into_iter().enumerate() {
            let mut child = match stage {
                PipelineStage::Ffmpeg { mut command } => {
                    // Log the exact command being executed
                    log::info!("Stage {}: Executing ffmpeg command: {:?}", i, command);
                    command.spawn().map_err(|e| {
                        log::error!("Failed to spawn ffmpeg process for stage {}: {}", i, e);
                        Error::IOError(e)
                    })?
                },
                PipelineStage::Sox { mut command } => {
                    log::info!("Stage {}: Executing sox command: {:?}", i, command);
                    command.spawn().map_err(|e| {
                        log::error!("Failed to spawn sox process for stage {}: {}", i, e);
                        Error::IOError(e)
                    })?
                },
            };
            
            // Set up piping between stages
            if i > 0 {
                // Get stdout from previous process and stdin for current process
                let prev_stdout = processes[i-1].stdout.take().ok_or_else(|| {
                    Error::InvalidInput(format!("Failed to get stdout from stage {}", i-1))
                })?;
                let curr_stdin = child.stdin.take().ok_or_else(|| {
                    Error::InvalidInput(format!("Failed to get stdin for stage {}", i))
                })?;
                
                // Spawn async task to pipe data between stages
                tokio::spawn(async move {
                    let mut reader = tokio::io::BufReader::new(prev_stdout);
                    let mut writer = curr_stdin;
                    match tokio::io::copy_buf(&mut reader, &mut writer).await {
                        Ok(bytes_copied) => log::debug!("Piped {} bytes between stages", bytes_copied),
                        Err(e) => log::error!("Error piping between stages: {}", e),
                    }
                });
            }
            
            processes.push(child);
        }
        
        // Return the final process for streaming
        let final_process = processes.pop().ok_or_else(|| {
            Error::InvalidInput("No final process to return".to_string())
        })?;
        
        // Spawn a cleanup task for intermediate processes
        tokio::spawn(async move {
            for (i, mut process) in processes.into_iter().enumerate() {
                match process.wait().await {
                    Ok(status) => {
                        if status.success() {
                            log::debug!("Stage {} completed successfully", i);
                        } else {
                            log::error!("Stage {} failed with exit code: {}", i, status.code().unwrap_or(-1));
                        }
                    },
                    Err(e) => log::error!("Error waiting for stage {}: {}", i, e),
                }
            }
        });
        
        log::info!("Pipeline execution started, returning final process for streaming");
        Ok(final_process)
    }
}

/// Audio effects processor that applies effects via real-time ffmpeg piping
pub struct AudioEffectsProcessor;

impl AudioEffectsProcessor {
    /// Create a new audio effects processor
    pub fn new() -> Result<Self, Error> {
        Ok(AudioEffectsProcessor)
    }

    /// Apply a chain of effects to an audio file using real-time streaming
    /// Returns the final streaming process for immediate consumption
    pub async fn apply_effects_streaming(&self, input_file: &Path, effects: &[AudioEffect]) -> Result<tokio::process::Child, Error> {
        log::info!("Applying {} effects to audio file: {:?}", effects.len(), input_file);
        for effect in effects {
            log::info!("  - Effect: {:?}", effect);
        }
        
        // Build the pipeline stages
        let mut pipeline = PipelineBuilder::new();
        
        // Always start with ffmpeg to decode input to WAV format
        let mut ffmpeg_cmd = tokio::process::Command::new("ffmpeg");
        ffmpeg_cmd.arg("-i").arg(input_file);
        
        // Separate sox effects from ffmpeg effects
        let has_reverb = effects.iter().any(|e| e.requires_sox());
        let ffmpeg_effects: Vec<_> = effects.iter().filter(|e| !e.requires_sox()).collect();
        
        log::info!("Pipeline configuration: has_reverb={}, ffmpeg_effects_count={}", has_reverb, ffmpeg_effects.len());
        
        // Stage 1: Start with ffmpeg for format conversion to PCM, optionally with effects
        if !has_reverb && !ffmpeg_effects.is_empty() {
            // If we only have ffmpeg effects and no reverb, apply them all in the first stage
            let filter_chain = ffmpeg_effects.iter()
                .map(|effect| effect.to_ffmpeg_filter())
                .collect::<Vec<_>>()
                .join(",");
            log::info!("Stage 1: ffmpeg with effects filter: {}", filter_chain);
            pipeline.add_ffmpeg_stage(ffmpeg_cmd, Some(filter_chain), "s16le")?;
        } else {
            // Always convert to PCM s16le - whether we have reverb or no effects
            log::info!("Stage 1: ffmpeg format conversion to PCM s16le");
            pipeline.add_ffmpeg_stage(ffmpeg_cmd, None, "s16le")?;
        }
        
        // Stage 2: Add sox stage if reverb is needed
        if has_reverb {
            log::info!("Stage 2: sox reverb processing");
            pipeline.add_sox_stage()?;
        }
        
        // Stage 3: Add ffmpeg effects stage if we have ffmpeg effects AND reverb
        // (if no reverb, the effects were already applied in stage 1)
        if has_reverb && !ffmpeg_effects.is_empty() {
            let filter_chain = ffmpeg_effects.iter()
                .map(|effect| effect.to_ffmpeg_filter())
                .collect::<Vec<_>>()
                .join(",");
            log::info!("Stage 3: ffmpeg with effects filter: {}", filter_chain);
            pipeline.add_ffmpeg_stage_with_input_pipe(Some(filter_chain))?;
        } else if has_reverb {
            // Only reverb, no additional processing needed since sox outputs PCM
            log::info!("Stage 3: No additional processing needed after sox");
        }
        
        log::info!("Executing pipeline with {} stages", pipeline.stages.len());
        
        // Execute the pipeline and return the streaming process
        pipeline.execute_streaming().await
    }
}

/// Parse a list of effect strings into AudioEffect enums
pub fn parse_effects(effect_strings: &[String]) -> Result<Vec<AudioEffect>, Error> {
    let mut effects = Vec::new();
    let mut unknown_effects = Vec::new();

    for effect_str in effect_strings {
        if let Some(effect) = AudioEffect::from_str(effect_str) {
            effects.push(effect);
        } else {
            unknown_effects.push(effect_str.clone());
        }
    }

    if !unknown_effects.is_empty() {
        return Err(Error::InvalidInput(format!(
            "Unknown effects: {}. Available effects: loud, fast, slow, reverb, echo, up, down, bass",
            unknown_effects.join(", ")
        )));
    }

    Ok(effects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_parsing() {
        assert_eq!(AudioEffect::from_str("loud"), Some(AudioEffect::Loud));
        assert_eq!(AudioEffect::from_str("FAST"), Some(AudioEffect::Fast));
        assert_eq!(AudioEffect::from_str("Reverb"), Some(AudioEffect::Reverb));
        assert_eq!(AudioEffect::from_str("bass"), Some(AudioEffect::Bass));
        assert_eq!(AudioEffect::from_str("BASS"), Some(AudioEffect::Bass));
        assert_eq!(AudioEffect::from_str("invalid"), None);
    }

    #[test]
    fn test_parse_effects() {
        let input = vec!["loud".to_string(), "fast".to_string(), "reverb".to_string(), "bass".to_string()];
        let effects = parse_effects(&input).unwrap();
        assert_eq!(effects, vec![AudioEffect::Loud, AudioEffect::Fast, AudioEffect::Reverb, AudioEffect::Bass]);

        let invalid = vec!["loud".to_string(), "invalid".to_string()];
        assert!(parse_effects(&invalid).is_err());
    }

    #[test]
    fn test_reverb_requires_sox() {
        assert!(AudioEffect::Reverb.requires_sox());
        assert!(!AudioEffect::Loud.requires_sox());
        assert!(!AudioEffect::Fast.requires_sox());
        assert!(!AudioEffect::Echo.requires_sox());
        assert!(!AudioEffect::Bass.requires_sox());
    }

    #[test]
    fn test_sox_effect_separation() {
        let effects = vec![AudioEffect::Loud, AudioEffect::Reverb, AudioEffect::Fast];
        let has_reverb = effects.iter().any(|e| e.requires_sox());
        let ffmpeg_effects: Vec<_> = effects.iter().filter(|e| !e.requires_sox()).collect();
        
        assert!(has_reverb);
        assert_eq!(ffmpeg_effects.len(), 2);
        assert_eq!(*ffmpeg_effects[0], AudioEffect::Loud);
        assert_eq!(*ffmpeg_effects[1], AudioEffect::Fast);
    }

    #[test]
    fn test_pipeline_selection() {
        // Test that the correct pipeline logic is selected
        let _processor = AudioEffectsProcessor::new().unwrap();
        
        // No effects should work
        let no_effects: Vec<AudioEffect> = vec![];
        let has_reverb = no_effects.iter().any(|e| e.requires_sox());
        assert!(!has_reverb);
        
        // Only ffmpeg effects
        let ffmpeg_only = vec![AudioEffect::Loud, AudioEffect::Fast, AudioEffect::Echo, AudioEffect::Bass];
        let has_reverb = ffmpeg_only.iter().any(|e| e.requires_sox());
        assert!(!has_reverb);
        
        // Mixed effects with reverb
        let mixed_effects = vec![AudioEffect::Loud, AudioEffect::Reverb, AudioEffect::Fast];
        let has_reverb = mixed_effects.iter().any(|e| e.requires_sox());
        let ffmpeg_effects: Vec<_> = mixed_effects.iter().filter(|e| !e.requires_sox()).collect();
        assert!(has_reverb);
        assert_eq!(ffmpeg_effects.len(), 2);
        
        // Only reverb
        let reverb_only = vec![AudioEffect::Reverb];
        let has_reverb = reverb_only.iter().any(|e| e.requires_sox());
        let ffmpeg_effects: Vec<_> = reverb_only.iter().filter(|e| !e.requires_sox()).collect();
        assert!(has_reverb);
        assert!(ffmpeg_effects.is_empty());
    }

    #[test]
    fn test_ffmpeg_filter_generation() {
        // Test that effects correctly generate ffmpeg filter strings
        assert_eq!(AudioEffect::Loud.to_ffmpeg_filter(), "volume=6dB");
        assert_eq!(AudioEffect::Fast.to_ffmpeg_filter(), "atempo=1.5");
        assert_eq!(AudioEffect::Slow.to_ffmpeg_filter(), "atempo=0.75");
        assert_eq!(AudioEffect::Echo.to_ffmpeg_filter(), "aecho=0.8:0.9:1000:0.3");
        assert_eq!(AudioEffect::Up.to_ffmpeg_filter(), "asetrate=48000*1.122462,aresample=48000");
        assert_eq!(AudioEffect::Down.to_ffmpeg_filter(), "asetrate=48000*0.890899,aresample=48000");
        assert_eq!(AudioEffect::Bass.to_ffmpeg_filter(), "equalizer=f=50:width_type=h:width=50:g=25");
        
        // Test filter chain construction
        let effects = vec![AudioEffect::Loud, AudioEffect::Fast];
        let filter_chain = effects.iter()
            .map(|effect| effect.to_ffmpeg_filter())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(filter_chain, "volume=6dB,atempo=1.5");
    }
}
