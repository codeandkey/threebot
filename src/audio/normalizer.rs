/// Volume normalization implementation using EBU R128 loudness measurement
///
/// This module provides volume normalization capabilities to maintain consistent
/// loudness levels across different audio sources. It uses a simplified implementation
/// of EBU R128 loudness measurement to calculate LUFS (Loudness Units relative to Full Scale).
use std::collections::VecDeque;

/// Simple volume normalizer using integrated loudness measurement
pub struct VolumeNormalizer {
    /// Target loudness in LUFS (Loudness Units relative to Full Scale)
    target_lufs: f32,
    /// Maximum gain boost allowed (in dB) to prevent over-amplification
    max_gain_db: f32,
    /// Running buffer for loudness measurement (keeps last ~400ms of audio)
    loudness_buffer: VecDeque<f32>,
    /// Buffer size for loudness measurement (in samples)
    buffer_size: usize,
    /// Current gain adjustment (linear multiplier)
    current_gain: f32,
    /// Smoothing factor for gain changes (0.0-1.0, smaller = slower adaptation)
    gain_smoothing: f32,
}

impl VolumeNormalizer {
    /// Create a new volume normalizer
    ///
    /// # Arguments
    /// * `target_lufs` - Target loudness in LUFS (typically -23 to -16)
    /// * `max_gain_db` - Maximum gain boost in dB (prevents over-amplification)
    /// * `sample_rate` - Audio sample rate in Hz
    pub fn new(target_lufs: f32, max_gain_db: f32, sample_rate: usize) -> Self {
        // Buffer size for ~400ms of stereo audio for loudness measurement
        let buffer_size = (sample_rate * 2 * 400) / 1000; // 400ms worth of stereo samples

        Self {
            target_lufs,
            max_gain_db,
            loudness_buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
            current_gain: 1.0,
            gain_smoothing: 0.01, // Slow adaptation to prevent pumping
        }
    }

    /// Process audio samples and apply volume normalization
    ///
    /// # Arguments
    /// * `samples` - Audio samples (interleaved stereo i16)
    ///
    /// Returns the processed samples with volume normalization applied
    pub fn process(&mut self, samples: &mut [i16]) {
        // Calculate integrated loudness for the current frame
        let frame_loudness = self.calculate_integrated_loudness(samples);

        // Update the loudness buffer
        self.loudness_buffer.push_back(frame_loudness);
        while self.loudness_buffer.len() > self.buffer_size {
            self.loudness_buffer.pop_front();
        }

        // Calculate the average loudness over the buffer period
        if !self.loudness_buffer.is_empty() {
            let avg_loudness =
                self.loudness_buffer.iter().sum::<f32>() / self.loudness_buffer.len() as f32;

            // Convert to LUFS (simplified approximation)
            let current_lufs = self.power_to_lufs(avg_loudness);

            // Calculate required gain adjustment
            let required_gain_db = self.target_lufs - current_lufs;
            let clamped_gain_db = required_gain_db.clamp(-20.0, self.max_gain_db);
            let target_gain = self.db_to_linear(clamped_gain_db);

            // Smooth the gain changes to prevent audible artifacts
            self.current_gain += (target_gain - self.current_gain) * self.gain_smoothing;
        }

        // Apply the gain to the samples
        if (self.current_gain - 1.0).abs() > 0.001 {
            // Only apply if gain is significantly different from 1.0
            for sample in samples.iter_mut() {
                let gained_sample = (*sample as f32 * self.current_gain) as i32;
                // Clamp to prevent overflow
                *sample = gained_sample.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }

    /// Calculate the integrated loudness of a frame (simplified implementation)
    fn calculate_integrated_loudness(&self, samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        // Convert samples to float and calculate mean square power
        let mut sum_squares = 0.0f64;
        for sample in samples {
            let float_sample = *sample as f64 / i16::MAX as f64;
            sum_squares += float_sample * float_sample;
        }

        // Return mean square power
        (sum_squares / samples.len() as f64) as f32
    }

    /// Convert power to LUFS (simplified approximation)
    fn power_to_lufs(&self, power: f32) -> f32 {
        if power <= 0.0 {
            -70.0 // Very quiet, assign low LUFS value
        } else {
            // Simplified conversion: LUFS = -0.691 + 10 * log10(power)
            // This is an approximation of the EBU R128 measurement
            -0.691 + 10.0 * power.log10()
        }
    }

    /// Convert dB to linear gain
    fn db_to_linear(&self, db: f32) -> f32 {
        10.0f32.powf(db / 20.0)
    }

    /// Get current gain multiplier (for debugging/monitoring)
    pub fn current_gain(&self) -> f32 {
        self.current_gain
    }

    /// Get current gain in dB (for debugging/monitoring)
    pub fn current_gain_db(&self) -> f32 {
        20.0 * self.current_gain.log10()
    }

    /// Reset the normalizer state
    pub fn reset(&mut self) {
        self.loudness_buffer.clear();
        self.current_gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_to_linear_conversion() {
        let normalizer = VolumeNormalizer::new(-18.0, 12.0, 48000);

        // Test known conversions with appropriate floating-point tolerance
        let result_0db = normalizer.db_to_linear(0.0);
        let result_6db = normalizer.db_to_linear(6.0);
        let result_neg6db = normalizer.db_to_linear(-6.0);

        // Use reasonable tolerance for floating-point comparison
        assert!((result_0db - 1.0).abs() < 0.0001, "0dB conversion failed");
        assert!(
            (result_6db - 2.0).abs() < 0.01,
            "6dB conversion failed: got {}, expected ~2.0",
            result_6db
        );
        assert!(
            (result_neg6db - 0.5).abs() < 0.01,
            "-6dB conversion failed: got {}, expected ~0.5",
            result_neg6db
        );
    }

    #[test]
    fn test_power_to_lufs() {
        let normalizer = VolumeNormalizer::new(-18.0, 12.0, 48000);

        // Test that zero power gives very low LUFS
        assert!(normalizer.power_to_lufs(0.0) < -60.0);

        // Test that higher power gives higher LUFS
        let low_power_lufs = normalizer.power_to_lufs(0.001);
        let high_power_lufs = normalizer.power_to_lufs(0.1);
        assert!(high_power_lufs > low_power_lufs);
    }

    #[test]
    fn test_normalizer_creation() {
        let normalizer = VolumeNormalizer::new(-18.0, 12.0, 48000);
        assert_eq!(normalizer.target_lufs, -18.0);
        assert_eq!(normalizer.max_gain_db, 12.0);
        assert_eq!(normalizer.current_gain, 1.0);
    }
}
