use std::{
    io::{self},
    process::Stdio,
    sync::Arc,
};

use log::trace;

use tokio::{
    io::AsyncReadExt,
    process::Command,
    sync::{Mutex, mpsc},
    time::{self, Duration},
};

use opus::Encoder;

use crate::{session::OutgoingMessage, util};

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
}

pub struct AudioMixerTask {
    streams: Arc<Mutex<Vec<AudioStream>>>,
    _task_handle: tokio::task::JoinHandle<()>,
}

impl AudioMixerTask {
    pub fn control(&self) -> AudioMixerControl {
        AudioMixerControl {
            streams: self.streams.clone(),
        }
    }
}

pub struct AudioMixer {
    streams: Arc<Mutex<Vec<AudioStream>>>,
    writer_sender: mpsc::Sender<OutgoingMessage>,
    encoder: Encoder,
    seq: u32,
}

impl AudioMixer {
    pub fn spawn(writer_sender: mpsc::Sender<OutgoingMessage>) -> AudioMixerTask {
        let mut mixer = AudioMixer::new(writer_sender);
        let streams = mixer.streams.clone();

        let task_handle = tokio::spawn(async move {
            mixer.mix_loop().await;
        });

        AudioMixerTask {
            streams,
            _task_handle: task_handle,
        }
    }

    pub fn new(writer_sender: mpsc::Sender<OutgoingMessage>) -> Self {
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
        };

        mixer
    }

    pub async fn mix_loop(&mut self) {
        let mut interval = time::interval(Duration::from_millis(FRAME_SIZE_MS));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let mut mixed: Vec<i16> = vec![0; FRAME_SAMPLES];
            let mut active = 0;

            let mut streams = self.streams.lock().await;
            streams.retain(|stream| {
                tokio::task::block_in_place(|| {
                    let mut pcm = futures::executor::block_on(stream.buffer.lock());
                    let is_finished = futures::executor::block_on(stream.finished.lock());

                    if pcm.len() < FRAME_SAMPLES {
                        if *is_finished && !pcm.is_empty() {
                            // Pad with zeros to complete the last frame
                            let mut padded = pcm.clone();
                            padded.resize(FRAME_SAMPLES, 0);
                            for i in 0..FRAME_SAMPLES {
                                mixed[i] = mixed[i].saturating_add(padded[i]);
                            }
                            pcm.clear();
                            active += 1;
                        }
                        // Remove finished streams with no data left
                        return !*is_finished || !pcm.is_empty();
                    }

                    for i in 0..FRAME_SAMPLES {
                        mixed[i] = mixed[i].saturating_add(pcm[i]);
                    }

                    pcm.drain(0..FRAME_SAMPLES);
                    active += 1;
                    true
                })
            });

            // If no active streams, don't bother encoding
            if active == 0 {
                continue;
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

            match self.encoder.encode(&mixed[..], &mut opus_buf[..]) {
                Ok(len) => {
                    opus_buf.truncate(len);
                },
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
            ].concat();

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
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let finished = Arc::new(Mutex::new(false));
        let buffer_clone = buffer.clone();
        let finished_clone = finished.clone();

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
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdout = child.stdout.take().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 512]; // 2 bytes per sample for i16
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
}