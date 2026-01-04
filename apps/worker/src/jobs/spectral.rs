//! FFT-based spectral analysis module
//!
//! Provides spectral feature extraction using pure-Rust FFT libraries.
//! Used for advanced audio analysis features like acousticness, speechiness, and valence.

// Allow dead_code until this module is integrated with feature_extraction.rs
#![allow(dead_code)]

use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex;

/// Default FFT frame size (2048 samples = ~46ms at 44.1kHz)
pub const DEFAULT_FRAME_SIZE: usize = 2048;

/// Default hop size (512 samples = ~11.6ms at 44.1kHz, 75% overlap)
pub const DEFAULT_HOP_SIZE: usize = 512;

/// Spectral analyzer with pre-computed FFT planner and window
pub struct SpectralAnalyzer {
    /// Real-to-complex FFT planner
    fft: Arc<dyn RealToComplex<f32>>,
    /// Pre-computed Hann window coefficients
    window: Vec<f32>,
    /// FFT frame size
    frame_size: usize,
    /// Hop size between frames
    hop_size: usize,
    /// Sample rate in Hz
    sample_rate: u32,
    /// Scratch buffer for FFT input
    scratch_input: Vec<f32>,
    /// Scratch buffer for FFT output
    scratch_output: Vec<Complex<f32>>,
}

impl SpectralAnalyzer {
    /// Create a new spectral analyzer with default parameters
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, DEFAULT_FRAME_SIZE, DEFAULT_HOP_SIZE)
    }

    /// Create a spectral analyzer with custom frame and hop sizes
    pub fn with_params(sample_rate: u32, frame_size: usize, hop_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(frame_size);

        // Pre-compute Hann window using apodize
        let window: Vec<f32> = apodize::hanning_iter(frame_size)
            .map(|x| x as f32)
            .collect();

        // Pre-allocate scratch buffers
        let scratch_input = vec![0.0f32; frame_size];
        let scratch_output = vec![Complex::new(0.0f32, 0.0f32); frame_size / 2 + 1];

        Self {
            fft,
            window,
            frame_size,
            hop_size,
            sample_rate,
            scratch_input,
            scratch_output,
        }
    }

    /// Get the frame size
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Get the hop size
    pub fn hop_size(&self) -> usize {
        self.hop_size
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Compute the magnitude spectrum of a frame
    ///
    /// Applies Hann windowing and FFT, returns magnitude spectrum.
    /// The input frame must be exactly `frame_size` samples.
    pub fn compute_spectrum(&mut self, frame: &[f32]) -> Vec<f32> {
        assert_eq!(
            frame.len(),
            self.frame_size,
            "Frame size mismatch: expected {}, got {}",
            self.frame_size,
            frame.len()
        );

        // Apply window and copy to scratch buffer
        for (i, (&sample, &window_coef)) in frame.iter().zip(self.window.iter()).enumerate() {
            self.scratch_input[i] = sample * window_coef;
        }

        // Perform FFT
        self.fft
            .process(&mut self.scratch_input, &mut self.scratch_output)
            .expect("FFT processing failed");

        // Compute magnitude spectrum
        self.scratch_output
            .iter()
            .map(|c| (c.re * c.re + c.im * c.im).sqrt())
            .collect()
    }

    /// Calculate spectral centroid (brightness measure)
    ///
    /// The spectral centroid is the weighted mean of frequencies,
    /// where the weights are the magnitudes. Returns frequency in Hz.
    pub fn spectral_centroid(&self, spectrum: &[f32]) -> f32 {
        let bin_width = self.sample_rate as f32 / (2.0 * spectrum.len() as f32);

        let mut weighted_sum = 0.0f32;
        let mut magnitude_sum = 0.0f32;

        for (i, &magnitude) in spectrum.iter().enumerate() {
            let frequency = i as f32 * bin_width;
            weighted_sum += frequency * magnitude;
            magnitude_sum += magnitude;
        }

        if magnitude_sum > f32::EPSILON {
            weighted_sum / magnitude_sum
        } else {
            0.0
        }
    }

    /// Calculate spectral flatness (tonality measure)
    ///
    /// Ratio of geometric mean to arithmetic mean of the spectrum.
    /// Returns 0.0 for pure tones, 1.0 for white noise.
    pub fn spectral_flatness(&self, spectrum: &[f32]) -> f32 {
        if spectrum.is_empty() {
            return 0.0;
        }

        // Filter out near-zero values to avoid log(0) issues
        let valid_magnitudes: Vec<f32> = spectrum
            .iter()
            .copied()
            .filter(|&m| m > f32::EPSILON)
            .collect();

        if valid_magnitudes.is_empty() {
            return 0.0;
        }

        let n = valid_magnitudes.len() as f32;

        // Geometric mean = exp(mean(log(x)))
        let log_sum: f32 = valid_magnitudes.iter().map(|&m| m.ln()).sum();
        let geometric_mean = (log_sum / n).exp();

        // Arithmetic mean
        let arithmetic_mean: f32 = valid_magnitudes.iter().sum::<f32>() / n;

        if arithmetic_mean > f32::EPSILON {
            (geometric_mean / arithmetic_mean).min(1.0)
        } else {
            0.0
        }
    }

    /// Calculate spectral rolloff frequency
    ///
    /// The frequency below which `percentile` percent of the total
    /// spectral energy is contained. Common values: 0.85 or 0.95.
    pub fn spectral_rolloff(&self, spectrum: &[f32], percentile: f32) -> f32 {
        if spectrum.is_empty() {
            return 0.0;
        }

        let bin_width = self.sample_rate as f32 / (2.0 * spectrum.len() as f32);

        // Calculate total energy (sum of squared magnitudes)
        let total_energy: f32 = spectrum.iter().map(|&m| m * m).sum();

        if total_energy < f32::EPSILON {
            return 0.0;
        }

        let threshold = total_energy * percentile.clamp(0.0, 1.0);
        let mut cumulative_energy = 0.0f32;

        for (i, &magnitude) in spectrum.iter().enumerate() {
            cumulative_energy += magnitude * magnitude;
            if cumulative_energy >= threshold {
                return (i as f32 + 1.0) * bin_width;
            }
        }

        // Return Nyquist if threshold not reached
        (spectrum.len() as f32) * bin_width
    }

    /// Calculate spectral flux (measure of spectral change)
    ///
    /// Sum of positive differences between consecutive spectra.
    /// Used for onset detection and rhythm analysis.
    pub fn spectral_flux(&self, prev_spectrum: &[f32], curr_spectrum: &[f32]) -> f32 {
        if prev_spectrum.len() != curr_spectrum.len() {
            return 0.0;
        }

        prev_spectrum
            .iter()
            .zip(curr_spectrum.iter())
            .map(|(&prev, &curr)| {
                let diff = curr - prev;
                if diff > 0.0 {
                    diff
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// Get frequency for a given bin index
    pub fn bin_to_frequency(&self, bin: usize) -> f32 {
        let bin_width = self.sample_rate as f32 / self.frame_size as f32;
        bin as f32 * bin_width
    }

    /// Get bin index for a given frequency
    pub fn frequency_to_bin(&self, frequency: f32) -> usize {
        let bin_width = self.sample_rate as f32 / self.frame_size as f32;
        (frequency / bin_width).round() as usize
    }

    /// Calculate energy in a frequency band
    pub fn band_energy(&self, spectrum: &[f32], low_hz: f32, high_hz: f32) -> f32 {
        let low_bin = self.frequency_to_bin(low_hz);
        let high_bin = self.frequency_to_bin(high_hz).min(spectrum.len() - 1);

        if low_bin >= spectrum.len() || low_bin > high_bin {
            return 0.0;
        }

        spectrum[low_bin..=high_bin]
            .iter()
            .map(|&m| m * m)
            .sum::<f32>()
            .sqrt()
    }
}

/// Calculate zero crossing rate of a signal
///
/// The rate at which the signal changes sign, normalized to [0, 1].
/// High ZCR indicates noisy/percussive sounds, low ZCR indicates tonal content.
pub fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }

    let mut crossings = 0u32;

    for window in samples.windows(2) {
        // Sign change occurs when product is negative
        if window[0] * window[1] < 0.0 {
            crossings += 1;
        }
    }

    // Normalize by number of possible crossings
    crossings as f32 / (samples.len() - 1) as f32
}

/// Aggregated spectral features from full audio analysis
#[derive(Debug, Clone, Default)]
pub struct SpectralFeatures {
    /// Mean spectral centroid (Hz)
    pub centroid_mean: f32,
    /// Standard deviation of spectral centroid
    pub centroid_std: f32,
    /// Mean spectral flatness (0-1)
    pub flatness_mean: f32,
    /// Mean spectral rolloff frequency (Hz)
    pub rolloff_mean: f32,
    /// Mean zero crossing rate (0-1)
    pub zcr_mean: f32,
    /// Standard deviation of zero crossing rate
    pub zcr_std: f32,
    /// Mean spectral flux
    pub spectral_flux_mean: f32,
    /// Ratio of high-frequency energy (above 4kHz) to total energy
    pub hf_energy_ratio: f32,
    /// Energy in vocal frequency band (300-3000 Hz)
    pub vocal_band_energy: f32,
}

/// Analyze spectral features of audio samples
///
/// Processes samples in overlapping frames and aggregates statistics.
pub fn analyze_spectral_features(samples: &[f32], sample_rate: u32) -> SpectralFeatures {
    if samples.is_empty() {
        return SpectralFeatures::default();
    }

    let mut analyzer = SpectralAnalyzer::new(sample_rate);
    let frame_size = analyzer.frame_size();
    let hop_size = analyzer.hop_size();

    if samples.len() < frame_size {
        // Not enough samples for even one frame
        return SpectralFeatures {
            zcr_mean: zero_crossing_rate(samples),
            ..Default::default()
        };
    }

    // Collect per-frame measurements
    let mut centroids: Vec<f32> = Vec::new();
    let mut flatnesses: Vec<f32> = Vec::new();
    let mut rolloffs: Vec<f32> = Vec::new();
    let mut zcrs: Vec<f32> = Vec::new();
    let mut fluxes: Vec<f32> = Vec::new();
    let mut hf_energies: Vec<f32> = Vec::new();
    let mut total_energies: Vec<f32> = Vec::new();
    let mut vocal_energies: Vec<f32> = Vec::new();

    let mut prev_spectrum: Option<Vec<f32>> = None;
    let mut frame_start = 0;

    while frame_start + frame_size <= samples.len() {
        let frame = &samples[frame_start..frame_start + frame_size];

        // Compute spectrum
        let spectrum = analyzer.compute_spectrum(frame);

        // Spectral features
        centroids.push(analyzer.spectral_centroid(&spectrum));
        flatnesses.push(analyzer.spectral_flatness(&spectrum));
        rolloffs.push(analyzer.spectral_rolloff(&spectrum, 0.85));

        // Zero crossing rate for this frame
        zcrs.push(zero_crossing_rate(frame));

        // Spectral flux (if we have a previous spectrum)
        if let Some(ref prev) = prev_spectrum {
            fluxes.push(analyzer.spectral_flux(prev, &spectrum));
        }

        // Energy calculations
        let total_energy: f32 = spectrum.iter().map(|&m| m * m).sum();
        total_energies.push(total_energy);

        // High frequency energy (above 4kHz)
        let hf_energy = analyzer.band_energy(&spectrum, 4000.0, sample_rate as f32 / 2.0);
        hf_energies.push(hf_energy * hf_energy); // Square to match total_energy units

        // Vocal band energy (300-3000 Hz)
        let vocal_energy = analyzer.band_energy(&spectrum, 300.0, 3000.0);
        vocal_energies.push(vocal_energy);

        prev_spectrum = Some(spectrum);
        frame_start += hop_size;
    }

    // Calculate aggregate statistics
    let centroid_mean = mean(&centroids);
    let centroid_std = std_dev(&centroids, centroid_mean);

    let flatness_mean = mean(&flatnesses);
    let rolloff_mean = mean(&rolloffs);

    let zcr_mean = mean(&zcrs);
    let zcr_std = std_dev(&zcrs, zcr_mean);

    let spectral_flux_mean = mean(&fluxes);

    // High frequency energy ratio
    let total_energy_sum: f32 = total_energies.iter().sum();
    let hf_energy_sum: f32 = hf_energies.iter().sum();
    let hf_energy_ratio = if total_energy_sum > f32::EPSILON {
        (hf_energy_sum / total_energy_sum).min(1.0)
    } else {
        0.0
    };

    // Mean vocal band energy (normalized)
    let vocal_band_energy = mean(&vocal_energies);

    SpectralFeatures {
        centroid_mean,
        centroid_std,
        flatness_mean,
        rolloff_mean,
        zcr_mean,
        zcr_std,
        spectral_flux_mean,
        hf_energy_ratio,
        vocal_band_energy,
    }
}

/// Calculate mean of a slice
fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

/// Calculate standard deviation given pre-computed mean
fn std_dev(values: &[f32], mean: f32) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance: f32 =
        values.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / values.len() as f32;
    variance.sqrt()
}

// ============================================================================
// High-Level Feature Computation Functions
// ============================================================================

/// Compute valence (musical positiveness) from spectral features
///
/// Valence is derived from brightness (spectral centroid) and tonality (inverse flatness).
/// High centroid and high tonality correlate with positive, happy-sounding music.
///
/// Returns a value in the range [0.0, 1.0]:
/// - High valence (>0.6): Bright, major-key, happy-sounding
/// - Low valence (<0.4): Dark, minor-key, sad-sounding
///
/// # Arguments
/// * `features` - Aggregated spectral features from audio analysis
/// * `_sample_rate` - Sample rate in Hz (unused, kept for API consistency)
pub fn compute_valence(features: &SpectralFeatures, _sample_rate: u32) -> f32 {
    // Brightness component: normalize centroid to 0-1 range
    // Typical vocal music has centroid between 1000-8000 Hz
    // Below 1000 Hz = very dark (bass-heavy), above 8000 Hz = very bright
    let centroid_normalized = (features.centroid_mean - 1000.0) / (8000.0 - 1000.0);
    let brightness = centroid_normalized.clamp(0.0, 1.0);

    // Tonality component: inverse of flatness
    // Low flatness = tonal (pure tones, harmonics) = positive
    // High flatness = noisy = less positive
    let tonality = 1.0 - features.flatness_mean.clamp(0.0, 1.0);

    // Weight brightness more heavily (60/40 split)
    let valence = 0.6 * brightness + 0.4 * tonality;

    valence.clamp(0.0, 1.0)
}

/// Compute acousticness (confidence that the track is acoustic) from spectral features
///
/// Acoustic music typically has:
/// - Low zero crossing rate (smooth waveforms vs. synthesized sounds)
/// - Low high-frequency energy (no electronic artifacts)
/// - Stable spectral content (low spectral flux)
///
/// Returns a value in the range [0.0, 1.0]:
/// - High acousticness (>0.7): Likely acoustic instruments, no electronic processing
/// - Low acousticness (<0.3): Likely electronic, synthesized, or heavily processed
pub fn compute_acousticness(features: &SpectralFeatures) -> f32 {
    // Low ZCR indicates acoustic sounds (smooth waveforms)
    // Electronic/synthesized sounds often have higher ZCR
    // Normalize: ZCR of 0.3 or higher is considered maximum "electronic"
    let zcr_score = 1.0 - (features.zcr_mean / 0.3).min(1.0);

    // Low high-frequency energy indicates acoustic (no synth harmonics/artifacts)
    let hf_score = 1.0 - features.hf_energy_ratio.clamp(0.0, 1.0);

    // Stable spectrum indicates acoustic (electronic often has rapid spectral changes)
    // Normalize: spectral flux of 0.5 or higher is considered maximum instability
    let stability = 1.0 - (features.spectral_flux_mean / 0.5).min(1.0);

    // Combine with roughly equal weights (35/35/30 split)
    let acousticness = 0.35 * zcr_score + 0.35 * hf_score + 0.30 * stability;

    acousticness.clamp(0.0, 1.0)
}

/// Compute instrumentalness (likelihood that the track contains no vocals)
///
/// Detects the presence of vocals by measuring energy in the vocal frequency band
/// (300-3000 Hz where human voice is most prominent).
///
/// Returns a value in the range [0.0, 1.0]:
/// - High instrumentalness (>0.7): Likely no vocals present
/// - Low instrumentalness (<0.3): Likely contains significant vocals
///
/// Note: This is a simplified heuristic. True vocal detection would require
/// more sophisticated techniques like harmonic-percussive separation or ML models.
pub fn compute_instrumentalness(features: &SpectralFeatures) -> f32 {
    // High energy in vocal band suggests vocals are present
    // Low energy in vocal band suggests instrumental
    // The vocal_band_energy is already computed as mean energy in 300-3000 Hz
    //
    // We invert and normalize: high vocal energy = low instrumentalness
    // Cap at 1.0 since vocal_band_energy can exceed 1.0 for loud signals
    let instrumentalness = 1.0 - features.vocal_band_energy.min(1.0);

    instrumentalness.clamp(0.0, 1.0)
}

/// Compute speechiness (presence of spoken words in the track)
///
/// Speech is characterized by:
/// - High variance in zero crossing rate (speech modulation patterns)
/// - Energy concentrated in vocal band but with rapid temporal changes
///
/// Returns a value in the range [0.0, 1.0]:
/// - High speechiness (>0.66): Likely spoken word, podcast, rap
/// - Medium speechiness (0.33-0.66): May contain both music and speech
/// - Low speechiness (<0.33): Likely music with little to no speech
///
/// Note: Differentiates from singing by using ZCR variance (speech has more
/// irregular modulation than sustained sung notes).
pub fn compute_speechiness(features: &SpectralFeatures) -> f32 {
    // High ZCR variance indicates speech-like modulation patterns
    // Singing tends to have more stable ZCR within phrases
    // Normalize: ZCR std of 0.15 or higher indicates significant speech-like variance
    let zcr_variance_score = (features.zcr_std / 0.15).min(1.0);

    // Moderate vocal band energy can indicate speech
    // Very high energy might be singing, very low is instrumental
    // We want moderate values, so use a different approach:
    // Low vocal energy = no speech (inverse relationship in this context)
    // The formula creates a bias toward detecting speech when ZCR variance is high
    let vocal_factor = 1.0 - features.vocal_band_energy.min(1.0);

    // Weight ZCR variance more heavily as it's the primary discriminator
    // The vocal_factor adds context but speech detection mainly relies on modulation
    let speechiness = 0.7 * zcr_variance_score + 0.3 * vocal_factor.abs();

    speechiness.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;

    /// Generate a pure sine wave at a given frequency
    fn generate_sine(frequency: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * frequency * t).sin()
            })
            .collect()
    }

    /// Generate white noise using LCG
    fn generate_noise(num_samples: usize, seed: u64) -> Vec<f32> {
        // Simple LCG for reproducible "random" noise (PCG-like)
        let mut state = seed.wrapping_add(1442695040888963407);
        (0..num_samples)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                // XorShift the upper bits and convert to float in [-1, 1]
                let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
                let rot = (state >> 59) as u32;
                let result = xorshifted.rotate_right(rot);
                // Convert u32 to float in [-1, 1]
                (result as f32 / u32::MAX as f32) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn test_spectral_analyzer_creation() {
        let analyzer = SpectralAnalyzer::new(44100);
        assert_eq!(analyzer.frame_size(), DEFAULT_FRAME_SIZE);
        assert_eq!(analyzer.hop_size(), DEFAULT_HOP_SIZE);
        assert_eq!(analyzer.sample_rate(), 44100);
    }

    #[test]
    fn test_spectral_centroid_sine_wave() {
        let sample_rate = 44100u32;
        let test_frequency = 1000.0f32; // 1kHz sine wave

        let samples = generate_sine(test_frequency, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // Centroid should be close to the sine wave frequency
        // Allow 5% tolerance due to windowing effects and bin resolution
        let tolerance = test_frequency * 0.05;
        assert!(
            (centroid - test_frequency).abs() < tolerance,
            "Expected centroid ~{} Hz, got {} Hz",
            test_frequency,
            centroid
        );
    }

    #[test]
    fn test_spectral_centroid_high_frequency() {
        let sample_rate = 44100u32;
        let test_frequency = 8000.0f32; // 8kHz sine wave

        let samples = generate_sine(test_frequency, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // Should be close to 8kHz
        let tolerance = test_frequency * 0.05;
        assert!(
            (centroid - test_frequency).abs() < tolerance,
            "Expected centroid ~{} Hz, got {} Hz",
            test_frequency,
            centroid
        );
    }

    #[test]
    fn test_spectral_flatness_sine_vs_noise() {
        let sample_rate = 44100u32;

        // Pure sine wave should have low flatness (tonal)
        let sine = generate_sine(440.0, sample_rate, DEFAULT_FRAME_SIZE);
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let sine_spectrum = analyzer.compute_spectrum(&sine);
        let sine_flatness = analyzer.spectral_flatness(&sine_spectrum);

        // Noise should have high flatness
        let noise = generate_noise(DEFAULT_FRAME_SIZE, 12345);
        let noise_spectrum = analyzer.compute_spectrum(&noise);
        let noise_flatness = analyzer.spectral_flatness(&noise_spectrum);

        assert!(
            sine_flatness < 0.1,
            "Sine wave flatness should be low, got {}",
            sine_flatness
        );
        assert!(
            noise_flatness > 0.3,
            "Noise flatness should be higher, got {}",
            noise_flatness
        );
        assert!(
            noise_flatness > sine_flatness,
            "Noise should be flatter than sine"
        );
    }

    #[test]
    fn test_spectral_rolloff() {
        let sample_rate = 44100u32;

        // Low frequency sine should have low rolloff
        let low_sine = generate_sine(200.0, sample_rate, DEFAULT_FRAME_SIZE);
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let low_spectrum = analyzer.compute_spectrum(&low_sine);
        let low_rolloff = analyzer.spectral_rolloff(&low_spectrum, 0.85);

        // High frequency sine should have high rolloff
        let high_sine = generate_sine(10000.0, sample_rate, DEFAULT_FRAME_SIZE);
        let high_spectrum = analyzer.compute_spectrum(&high_sine);
        let high_rolloff = analyzer.spectral_rolloff(&high_spectrum, 0.85);

        assert!(
            high_rolloff > low_rolloff,
            "High freq rolloff {} should be > low freq rolloff {}",
            high_rolloff,
            low_rolloff
        );
    }

    #[test]
    fn test_spectral_flux() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Same spectrum should have zero flux
        let samples = generate_sine(440.0, sample_rate, DEFAULT_FRAME_SIZE);
        let spectrum = analyzer.compute_spectrum(&samples);
        let same_flux = analyzer.spectral_flux(&spectrum, &spectrum);
        assert!(
            same_flux < f32::EPSILON,
            "Same spectrum flux should be ~0, got {}",
            same_flux
        );

        // Different spectra should have non-zero flux
        let samples2 = generate_sine(880.0, sample_rate, DEFAULT_FRAME_SIZE);
        let spectrum2 = analyzer.compute_spectrum(&samples2);
        let diff_flux = analyzer.spectral_flux(&spectrum, &spectrum2);
        assert!(
            diff_flux > 0.0,
            "Different spectra should have non-zero flux"
        );
    }

    #[test]
    fn test_zero_crossing_rate_sine() {
        let sample_rate = 44100u32;

        // Higher frequency = more zero crossings
        let low_freq = generate_sine(100.0, sample_rate, 4410); // 0.1 second
        let high_freq = generate_sine(1000.0, sample_rate, 4410);

        let low_zcr = zero_crossing_rate(&low_freq);
        let high_zcr = zero_crossing_rate(&high_freq);

        assert!(
            high_zcr > low_zcr,
            "Higher frequency should have higher ZCR: {} vs {}",
            high_zcr,
            low_zcr
        );

        // 100 Hz at 44100 sample rate: ~2 crossings per period, ~100 periods/sec
        // Expected ZCR ≈ 200 / 44100 ≈ 0.0045
        assert!(
            low_zcr > 0.001 && low_zcr < 0.01,
            "100 Hz ZCR should be around 0.0045, got {}",
            low_zcr
        );
    }

    #[test]
    fn test_zero_crossing_rate_noise() {
        let noise = generate_noise(4410, 54321);
        let zcr = zero_crossing_rate(&noise);

        // Noise should have relatively high ZCR (around 0.5 for random)
        assert!(
            zcr > 0.2 && zcr < 0.8,
            "Noise ZCR should be moderate to high, got {}",
            zcr
        );
    }

    #[test]
    fn test_zero_crossing_rate_edge_cases() {
        assert_eq!(zero_crossing_rate(&[]), 0.0);
        assert_eq!(zero_crossing_rate(&[1.0]), 0.0);
        assert_eq!(zero_crossing_rate(&[1.0, -1.0]), 1.0); // One crossing
        assert_eq!(zero_crossing_rate(&[1.0, 1.0]), 0.0); // No crossing
    }

    #[test]
    fn test_analyze_spectral_features_sine() {
        let sample_rate = 44100u32;
        let samples = generate_sine(1000.0, sample_rate, sample_rate as usize); // 1 second

        let features = analyze_spectral_features(&samples, sample_rate);

        // Centroid should be around 1000 Hz
        assert!(
            (features.centroid_mean - 1000.0).abs() < 100.0,
            "Centroid mean should be ~1000 Hz, got {}",
            features.centroid_mean
        );

        // Pure tone should have low flatness
        assert!(
            features.flatness_mean < 0.1,
            "Sine flatness should be low, got {}",
            features.flatness_mean
        );
    }

    #[test]
    fn test_analyze_spectral_features_empty() {
        let features = analyze_spectral_features(&[], 44100);
        assert_eq!(features.centroid_mean, 0.0);
        assert_eq!(features.flatness_mean, 0.0);
    }

    #[test]
    fn test_analyze_spectral_features_short() {
        // Fewer samples than one frame
        let samples = generate_sine(440.0, 44100, 100);
        let features = analyze_spectral_features(&samples, 44100);

        // Should still compute ZCR even for short samples
        assert!(features.zcr_mean > 0.0);
    }

    #[test]
    fn test_band_energy() {
        let sample_rate = 44100u32;

        // Generate 1kHz sine
        let samples = generate_sine(1000.0, sample_rate, DEFAULT_FRAME_SIZE);
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);

        // Energy should be concentrated around 1kHz
        let mid_energy = analyzer.band_energy(&spectrum, 800.0, 1200.0);
        let high_energy = analyzer.band_energy(&spectrum, 5000.0, 10000.0);

        assert!(
            mid_energy > high_energy,
            "1kHz sine should have more energy in 800-1200Hz ({}) than 5-10kHz ({})",
            mid_energy,
            high_energy
        );
    }

    #[test]
    fn test_bin_frequency_conversion() {
        let analyzer = SpectralAnalyzer::new(44100);

        // At 44100 Hz sample rate with 2048 frame size:
        // Bin resolution = 44100 / 2048 ≈ 21.5 Hz per bin
        let freq = analyzer.bin_to_frequency(46); // Should be ~990 Hz
        assert!(
            (freq - 990.0).abs() < 50.0,
            "Bin 46 should be ~990 Hz, got {}",
            freq
        );

        let bin = analyzer.frequency_to_bin(1000.0);
        assert!(
            (bin as i32 - 46).abs() <= 1,
            "1000 Hz should be ~bin 46, got {}",
            bin
        );
    }

    // ========================================================================
    // Tests for High-Level Feature Computation Functions
    // ========================================================================

    #[test]
    fn test_compute_valence_high_centroid() {
        // High spectral centroid (bright sound) should yield high valence
        let features = SpectralFeatures {
            centroid_mean: 6000.0, // High centroid = bright
            flatness_mean: 0.1,    // Low flatness = tonal
            ..Default::default()
        };

        let valence = compute_valence(&features, 44100);

        assert!(
            valence > 0.5,
            "High centroid should produce high valence, got {}",
            valence
        );
        assert!(
            valence <= 1.0,
            "Valence should be clamped to 1.0, got {}",
            valence
        );
    }

    #[test]
    fn test_compute_valence_low_centroid() {
        // Low spectral centroid (dark sound) should yield low valence
        let features = SpectralFeatures {
            centroid_mean: 500.0, // Low centroid = dark
            flatness_mean: 0.5,   // Moderate flatness
            ..Default::default()
        };

        let valence = compute_valence(&features, 44100);

        assert!(
            valence < 0.5,
            "Low centroid should produce low valence, got {}",
            valence
        );
        assert!(
            valence >= 0.0,
            "Valence should be clamped to 0.0, got {}",
            valence
        );
    }

    #[test]
    fn test_compute_valence_range() {
        // Test that output is always in [0.0, 1.0] range
        let test_cases = [
            (0.0, 0.0),       // Minimum values
            (1000.0, 0.0),    // Edge case: exactly at low boundary
            (8000.0, 0.0),    // Edge case: exactly at high boundary
            (10000.0, 1.0),   // Beyond max centroid
            (5000.0, 0.5),    // Mid range
        ];

        for (centroid, flatness) in test_cases {
            let features = SpectralFeatures {
                centroid_mean: centroid,
                flatness_mean: flatness,
                ..Default::default()
            };
            let valence = compute_valence(&features, 44100);

            assert!(
                (0.0..=1.0).contains(&valence),
                "Valence must be in [0.0, 1.0], got {} for centroid={}, flatness={}",
                valence,
                centroid,
                flatness
            );
        }
    }

    #[test]
    fn test_compute_acousticness_low_zcr_low_hf() {
        // Low ZCR and low HF energy should indicate acoustic content
        let features = SpectralFeatures {
            zcr_mean: 0.02,           // Very low ZCR = acoustic
            hf_energy_ratio: 0.05,    // Low HF = acoustic
            spectral_flux_mean: 0.05, // Low flux = stable/acoustic
            ..Default::default()
        };

        let acousticness = compute_acousticness(&features);

        assert!(
            acousticness > 0.7,
            "Low ZCR + low HF should produce high acousticness, got {}",
            acousticness
        );
    }

    #[test]
    fn test_compute_acousticness_high_zcr_high_hf() {
        // High ZCR and high HF energy should indicate electronic content
        let features = SpectralFeatures {
            zcr_mean: 0.4,           // High ZCR = electronic
            hf_energy_ratio: 0.8,    // High HF = electronic
            spectral_flux_mean: 0.6, // High flux = unstable/electronic
            ..Default::default()
        };

        let acousticness = compute_acousticness(&features);

        assert!(
            acousticness < 0.3,
            "High ZCR + high HF should produce low acousticness, got {}",
            acousticness
        );
    }

    #[test]
    fn test_compute_acousticness_range() {
        // Test edge cases for range clamping
        let test_cases = [
            (0.0, 0.0, 0.0),     // All minimum = max acousticness
            (0.5, 1.0, 1.0),     // All maximum = min acousticness
            (0.15, 0.5, 0.25),   // Mixed values
        ];

        for (zcr, hf, flux) in test_cases {
            let features = SpectralFeatures {
                zcr_mean: zcr,
                hf_energy_ratio: hf,
                spectral_flux_mean: flux,
                ..Default::default()
            };
            let acousticness = compute_acousticness(&features);

            assert!(
                (0.0..=1.0).contains(&acousticness),
                "Acousticness must be in [0.0, 1.0], got {} for zcr={}, hf={}, flux={}",
                acousticness,
                zcr,
                hf,
                flux
            );
        }
    }

    #[test]
    fn test_compute_instrumentalness_low_vocal_energy() {
        // Low vocal band energy should indicate instrumental content
        let features = SpectralFeatures {
            vocal_band_energy: 0.1, // Low energy in vocal band
            ..Default::default()
        };

        let instrumentalness = compute_instrumentalness(&features);

        assert!(
            instrumentalness > 0.8,
            "Low vocal energy should produce high instrumentalness, got {}",
            instrumentalness
        );
    }

    #[test]
    fn test_compute_instrumentalness_high_vocal_energy() {
        // High vocal band energy should indicate vocals present
        let features = SpectralFeatures {
            vocal_band_energy: 0.9, // High energy in vocal band
            ..Default::default()
        };

        let instrumentalness = compute_instrumentalness(&features);

        assert!(
            instrumentalness < 0.2,
            "High vocal energy should produce low instrumentalness, got {}",
            instrumentalness
        );
    }

    #[test]
    fn test_compute_instrumentalness_range() {
        // Test edge cases including values > 1.0 (which can occur with loud signals)
        let test_cases = [0.0, 0.5, 1.0, 1.5, 2.0];

        for vocal_energy in test_cases {
            let features = SpectralFeatures {
                vocal_band_energy: vocal_energy,
                ..Default::default()
            };
            let instrumentalness = compute_instrumentalness(&features);

            assert!(
                (0.0..=1.0).contains(&instrumentalness),
                "Instrumentalness must be in [0.0, 1.0], got {} for vocal_energy={}",
                instrumentalness,
                vocal_energy
            );
        }
    }

    #[test]
    fn test_compute_speechiness_high_zcr_variance() {
        // High ZCR variance should indicate speech-like modulation
        let features = SpectralFeatures {
            zcr_std: 0.2,           // High variance in ZCR
            vocal_band_energy: 0.3, // Some vocal energy
            ..Default::default()
        };

        let speechiness = compute_speechiness(&features);

        assert!(
            speechiness > 0.5,
            "High ZCR variance should produce higher speechiness, got {}",
            speechiness
        );
    }

    #[test]
    fn test_compute_speechiness_low_zcr_variance() {
        // Low ZCR variance should indicate sustained tones (singing, not speech)
        let features = SpectralFeatures {
            zcr_std: 0.01,          // Very low variance
            vocal_band_energy: 0.8, // High vocal energy (singing)
            ..Default::default()
        };

        let speechiness = compute_speechiness(&features);

        assert!(
            speechiness < 0.3,
            "Low ZCR variance should produce low speechiness, got {}",
            speechiness
        );
    }

    #[test]
    fn test_compute_speechiness_range() {
        // Test edge cases
        let test_cases = [
            (0.0, 0.0),
            (0.15, 0.5),
            (0.3, 1.0),
            (0.0, 1.5), // vocal_band_energy > 1.0
        ];

        for (zcr_std, vocal) in test_cases {
            let features = SpectralFeatures {
                zcr_std,
                vocal_band_energy: vocal,
                ..Default::default()
            };
            let speechiness = compute_speechiness(&features);

            assert!(
                (0.0..=1.0).contains(&speechiness),
                "Speechiness must be in [0.0, 1.0], got {} for zcr_std={}, vocal={}",
                speechiness,
                zcr_std,
                vocal
            );
        }
    }

    #[test]
    fn test_feature_functions_with_real_audio() {
        // Integration test: generate audio signals and compute features
        let sample_rate = 44100u32;

        // High-frequency sine wave should have high valence (bright)
        let bright_samples = generate_sine(6000.0, sample_rate, sample_rate as usize);
        let bright_features = analyze_spectral_features(&bright_samples, sample_rate);
        let bright_valence = compute_valence(&bright_features, sample_rate);

        // Low-frequency sine wave should have lower valence (dark)
        let dark_samples = generate_sine(200.0, sample_rate, sample_rate as usize);
        let dark_features = analyze_spectral_features(&dark_samples, sample_rate);
        let dark_valence = compute_valence(&dark_features, sample_rate);

        assert!(
            bright_valence > dark_valence,
            "Bright audio ({}) should have higher valence than dark audio ({})",
            bright_valence,
            dark_valence
        );

        // Low-frequency sine wave (less ZCR) should be more acoustic than noise
        // Note: High-frequency sines have many zero crossings and thus may appear less acoustic
        let low_sine_acousticness = compute_acousticness(&dark_features);
        let noise = generate_noise(sample_rate as usize, 99999);
        let noise_features = analyze_spectral_features(&noise, sample_rate);
        let noise_acousticness = compute_acousticness(&noise_features);

        assert!(
            low_sine_acousticness > noise_acousticness,
            "Low-frequency sine ({}) should be more acoustic than noise ({})",
            low_sine_acousticness,
            noise_acousticness
        );

        // Low vocal band energy should produce high instrumentalness
        // A high-frequency sine (above 3kHz) has less vocal band energy
        let high_freq_instrumentalness = compute_instrumentalness(&bright_features);
        // A mid-frequency sine in the vocal band
        let mid_samples = generate_sine(1000.0, sample_rate, sample_rate as usize);
        let mid_features = analyze_spectral_features(&mid_samples, sample_rate);
        let mid_freq_instrumentalness = compute_instrumentalness(&mid_features);

        assert!(
            high_freq_instrumentalness > mid_freq_instrumentalness,
            "High-freq sine ({}) should have higher instrumentalness than mid-freq ({})",
            high_freq_instrumentalness,
            mid_freq_instrumentalness
        );
    }
}
