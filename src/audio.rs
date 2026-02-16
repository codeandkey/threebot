use std::{
    io::{self},
    path::Path,
    process::Stdio,
    sync::Arc,
};

use log::trace;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt},
    process::Command,
    sync::{Mutex, mpsc},
    time::{self, Duration},
};

use opus::Encoder;

use crate::{
    config::{AudioEffectSettings, BehaviorSettings},
    session::OutgoingMessage,
    util,
};
use effects::{AudioEffect, AudioEffectsProcessor};

pub mod effects;
pub mod normalizer;

use normalizer::VolumeNormalizer;

const SAMPLE_RATE: usize = 48000;
const CHANNELS: usize = 2;
const FRAME_SAMPLES: usize = 960 * CHANNELS;
const FRAME_SIZE_MS: u64 = 20;

struct AudioStream {
    buffer: Arc<Mutex<Vec<i16>>>,
    finished: Arc<Mutex<bool>>,
}

pub struct AudioMixerControl {
    streams: Arc<Mutex<Vec<AudioStream>>>,
    audio_effects: AudioEffectSettings,
    audio_buffer_size: usize,
}

pub struct AudioMixerTask {
    streams: Arc<Mutex<Vec<AudioStream>>>,
    audio_effects: AudioEffectSettings,
    audio_buffer_size: usize,
    _task_handle: tokio::task::JoinHandle<()>,
}

impl AudioMixerTask {
    pub fn control(&self) -> AudioMixerControl {
        AudioMixerControl {
            streams: self.streams.clone(),
            audio_effects: self.audio_effects.clone(),
            audio_buffer_size: self.audio_buffer_size,
        }
    }
}

pub struct AudioMixer {
    streams: Arc<Mutex<Vec<AudioStream>>>,
    writer_sender: mpsc::Sender<OutgoingMessage>,
    encoder: Encoder,
    seq: u32,
    volume: f32,
    normalizer: Option<VolumeNormalizer>,
    audio_effects: AudioEffectSettings,
    // Pre-allocated buffers to reduce allocations in hot path
    mixed_buffer: Vec<i16>,
    temp_buffer: Vec<i16>,
}

impl AudioMixer {
    pub fn spawn(
        writer_sender: mpsc::Sender<OutgoingMessage>,
        behavior_settings: &BehaviorSettings,
        audio_effects: &AudioEffectSettings,
    ) -> AudioMixerTask {
        let mut mixer = AudioMixer::new(writer_sender, behavior_settings, audio_effects);
        let streams = mixer.streams.clone();

        let task_handle = tokio::spawn(async move {
            mixer.mix_loop().await;
        });

        AudioMixerTask {
            streams,
            audio_effects: audio_effects.clone(),
            audio_buffer_size: behavior_settings.audio_buffer_size,
            _task_handle: task_handle,
        }
    }

    pub fn new(
        writer_sender: mpsc::Sender<OutgoingMessage>,
        behavior_settings: &BehaviorSettings,
        audio_effects: &AudioEffectSettings,
    ) -> Self {
        let normalizer = if behavior_settings.volume_normalization_enabled {
            Some(VolumeNormalizer::new(
                behavior_settings.target_loudness_lufs,
                behavior_settings.max_normalization_gain_db,
                SAMPLE_RATE,
            ))
        } else {
            None
        };

        let mixer = AudioMixer {
            streams: Arc::new(Mutex::new(Vec::new())),
            writer_sender,
            encoder: Encoder::new(
                SAMPLE_RATE.try_into().unwrap(),
                opus::Channels::Stereo,
                opus::Application::Voip,
            )
            .unwrap(),
            seq: 0,
            volume: behavior_settings.volume,
            normalizer,
            audio_effects: audio_effects.clone(),
            // Pre-allocate buffers for better performance
            mixed_buffer: vec![0; FRAME_SAMPLES],
            temp_buffer: Vec::with_capacity(FRAME_SAMPLES),
        };

        mixer
    }

    pub async fn mix_loop(&mut self) {
        let mut interval = time::interval(Duration::from_millis(FRAME_SIZE_MS));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            // Reuse pre-allocated buffers instead of allocating new ones
            self.mixed_buffer.fill(0);
            let mut active = 0;

            // Pre-allocate vectors to reduce allocations in hot path
            let mut streams_to_remove = Vec::new();

            {
                let mut streams = self.streams.lock().await;

                for (stream_index, stream) in streams.iter().enumerate() {
                    // Try to acquire locks without blocking - use try_lock for better performance
                    if let Ok(mut pcm) = stream.buffer.try_lock() {
                        if let Ok(is_finished) = stream.finished.try_lock() {
                            if pcm.len() < FRAME_SAMPLES {
                                if *is_finished && !pcm.is_empty() {
                                    // Pad with zeros to complete the last frame
                                    self.temp_buffer.clear();
                                    self.temp_buffer.extend_from_slice(&pcm);
                                    self.temp_buffer.resize(FRAME_SAMPLES, 0);

                                    for i in 0..FRAME_SAMPLES {
                                        self.mixed_buffer[i] = self.mixed_buffer[i]
                                            .saturating_add(self.temp_buffer[i]);
                                    }
                                    pcm.clear();
                                    active += 1;
                                    streams_to_remove.push(stream_index);
                                } else if *is_finished {
                                    streams_to_remove.push(stream_index);
                                }
                                continue;
                            }

                            // Process full frame
                            for i in 0..FRAME_SAMPLES {
                                self.mixed_buffer[i] = self.mixed_buffer[i].saturating_add(pcm[i]);
                            }

                            pcm.drain(0..FRAME_SAMPLES);
                            active += 1;
                        }
                    }
                }

                // Remove finished streams (iterate in reverse to maintain indices)
                for &index in streams_to_remove.iter().rev() {
                    streams.remove(index);
                }
            }

            // If no active streams, don't bother encoding
            if active == 0 {
                continue;
            }

            // Apply volume normalization if enabled
            if let Some(ref mut normalizer) = self.normalizer {
                normalizer.process(&mut self.mixed_buffer);
            }

            // Apply global volume multiplier to the mixed audio
            if self.volume != 1.0 {
                // Use integer arithmetic when possible for better performance
                if self.volume == 0.5 {
                    // Common case: half volume can use bit shifting
                    for sample in self.mixed_buffer.iter_mut() {
                        *sample = *sample >> 1;
                    }
                } else if self.volume == 2.0 {
                    // Common case: double volume with saturation
                    for sample in self.mixed_buffer.iter_mut() {
                        let doubled = (*sample as i32) << 1;
                        *sample = doubled.max(i16::MIN as i32).min(i16::MAX as i32) as i16;
                    }
                } else {
                    // General case: use floating point
                    for sample in self.mixed_buffer.iter_mut() {
                        let scaled_sample = (*sample as f32 * self.volume) as i32;
                        // Clamp to i16 range to prevent overflow
                        *sample = scaled_sample.max(i16::MIN as i32).min(i16::MAX as i32) as i16;
                    }
                }
            }

            // Opus frame structure:
            // 1 byte header
            // varint sequence number
            // opus payload
            // positional data

            self.seq = self.seq.wrapping_add(2); // any other values cause choppy audio. what?

            let header_byte = 0b1000_0000 as u8; // First bit indicates OPUS encoding
            let seq = util::encode_varint_long(self.seq as u64);
            let mut opus_buf = vec![0; 1000];

            match self
                .encoder
                .encode(&self.mixed_buffer[..], &mut opus_buf[..])
            {
                Ok(len) => {
                    opus_buf.truncate(len);
                }
                Err(e) => {
                    eprintln!("Failed to encode audio: {}", e);
                    continue;
                }
            }

            let opus_header_value = opus_buf.len() as u64;
            let mut opus_header = util::encode_varint_long(opus_header_value);

            opus_header[0] |= 0x20; // Force termination bit

            let final_frame = [
                &[header_byte],
                seq.as_slice(),
                opus_header.as_slice(),
                opus_buf.as_slice(),
            ]
            .concat();

            if let Err(e) = self
                .writer_sender
                .send(OutgoingMessage::AudioData(final_frame))
                .await
            {
                eprintln!("Failed to send audio data: {}", e);
                break;
            }

            trace!("Wrote audio frame with sequence number {}", self.seq);
        }
    }
}

impl AudioMixerControl {
    pub async fn play_sound(&self, file: &str) -> io::Result<()> {
        self.play_sound_with_effects(file, &[]).await
    }

    pub async fn play_sound_with_effects(
        &self,
        file: &str,
        effects: &[AudioEffect],
    ) -> io::Result<()> {
        log::info!("Playing sound {} with {} effects", file, effects.len());
        for (i, effect) in effects.iter().enumerate() {
            log::info!("  Effect {}: {:?}", i, effect);
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let finished = Arc::new(Mutex::new(false));
        let buffer_clone = buffer.clone();
        let finished_clone = finished.clone();

        // Create the processing pipeline
        let mut child = if !effects.is_empty() {
            log::info!("Using effects pipeline for {} effects", effects.len());
            // Apply effects using the streaming processor - this now outputs PCM s16le directly
            let processor = AudioEffectsProcessor::new(self.audio_effects.clone())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            // Get the streaming process with effects applied
            // The effects pipeline now outputs the final PCM format directly
            processor
                .apply_effects_streaming(Path::new(file), effects)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        } else {
            log::info!("Using direct ffmpeg conversion (no effects)");
            // No effects, use original file directly
            let mut child = Command::new("ffmpeg")
                .args([
                    "-i",
                    file,
                    "-f",
                    "s16le",
                    "-acodec",
                    "pcm_s16le",
                    "-ar",
                    &SAMPLE_RATE.to_string(),
                    "-ac",
                    &CHANNELS.to_string(),
                    "-",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()) // Capture stderr for potential error reporting
                .spawn()?;

            // Capture stderr in background task (don't log immediately)
            let stderr_handle = if let Some(stderr) = child.stderr.take() {
                Some(tokio::spawn(async move {
                    let mut reader = tokio::io::BufReader::new(stderr);
                    let mut line = String::new();
                    let mut lines = Vec::new();
                    while let Ok(n) = reader.read_line(&mut line).await {
                        if n == 0 {
                            break;
                        }
                        lines.push(format!("FFmpeg direct conversion stderr: {}", line.trim()));
                        line.clear();
                    }
                    lines
                }))
            } else {
                None
            };

            // Store stderr handle for later status checking
            let stderr_handle_for_status = stderr_handle;

            // Spawn task to monitor process and log stderr on apparent failure
            tokio::spawn(async move {
                // Wait for stderr collection to complete
                // Note: We can't directly wait on the child here since stdout is being consumed
                // Instead, we check stderr content for error indicators
                if let Some(handle) = stderr_handle_for_status {
                    match handle.await {
                        Ok(stderr_lines) => {
                            // Check if there are any error indicators in stderr
                            let has_errors = stderr_lines.iter().any(|line| {
                                line.contains("Error")
                                    || line.contains("failed")
                                    || line.contains("Invalid")
                            });

                            // Log stderr if there were apparent errors
                            if has_errors {
                                log::error!("FFmpeg direct conversion appears to have failed:");
                                for stderr_line in stderr_lines {
                                    log::error!("{}", stderr_line);
                                }
                            }
                        }
                        Err(e) => log::error!("Failed to collect FFmpeg stderr: {}", e),
                    }
                }
            });

            child
        };

        let mut stdout = child.stdout.take().unwrap();
        let buffer_size = self.audio_buffer_size;
        tokio::spawn(async move {
            let mut buf = vec![0u8; buffer_size]; // Use configurable buffer size
            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut pcm = buffer_clone.lock().await;
                        for chunk in buf[..n].chunks_exact(2) {
                            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                            pcm.push(sample);
                        }
                    }
                    Err(_) => break,
                }
            }
            *finished_clone.lock().await = true;
        });

        let mut streams = self.streams.lock().await;
        streams.push(AudioStream { buffer, finished });

        Ok(())
    }

    pub async fn stop_all_streams(&self) {
        log::info!("Stopping all audio streams");
        let mut streams = self.streams.lock().await;
        streams.clear();
    }
}
