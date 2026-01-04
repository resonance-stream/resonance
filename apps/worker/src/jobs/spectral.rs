//! FFT-based spectral analysis module
//!
//! Provides spectral feature extraction using pure-Rust FFT libraries.
//! Used for advanced audio analysis features like acousticness, speechiness, and valence.

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
#[allow(dead_code)]
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
    let instrumentalness = 1.0 - features.vocal_band_energy.clamp(0.0, 1.0);

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

    /// Generate a signal with multiple harmonics (fundamental + overtones)
    fn generate_harmonics(
        fundamental: f32,
        harmonics: &[(u32, f32)], // (harmonic number, amplitude)
        sample_rate: u32,
        num_samples: usize,
    ) -> Vec<f32> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let mut sample = (2.0 * PI * fundamental * t).sin(); // Fundamental
                for &(harmonic_num, amplitude) in harmonics {
                    let freq = fundamental * harmonic_num as f32;
                    sample += amplitude * (2.0 * PI * freq * t).sin();
                }
                sample
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

    /// Generate pink noise (1/f power spectrum) using Voss-McCartney algorithm
    /// Pink noise has equal energy per octave, unlike white noise
    fn generate_pink_noise(num_samples: usize, seed: u64) -> Vec<f32> {
        const NUM_ROWS: usize = 16;
        let mut rows = [0.0f32; NUM_ROWS];
        let mut running_sum = 0.0f32;

        // LCG state for random generation
        let mut state = seed.wrapping_add(1442695040888963407);
        let mut next_random = || -> f32 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
            let rot = (state >> 59) as u32;
            let result = xorshifted.rotate_right(rot);
            (result as f32 / u32::MAX as f32) * 2.0 - 1.0
        };

        // Initialize rows
        for row in rows.iter_mut() {
            let val = next_random();
            *row = val;
            running_sum += val;
        }

        (0..num_samples)
            .map(|i| {
                // Voss-McCartney: update one row per sample based on trailing zeros in index
                let num_zeros = (i + 1).trailing_zeros() as usize;
                if num_zeros < NUM_ROWS {
                    running_sum -= rows[num_zeros];
                    let new_val = next_random();
                    running_sum += new_val;
                    rows[num_zeros] = new_val;
                }

                // Add white noise component and normalize
                let white = next_random();
                (running_sum + white) / (NUM_ROWS as f32 + 1.0)
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

    // ========================================================================
    // Comprehensive Spectral Centroid Tests
    // ========================================================================

    #[test]
    fn test_spectral_centroid_100hz() {
        let sample_rate = 44100u32;
        let test_frequency = 100.0f32;

        let samples = generate_sine(test_frequency, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // For low frequencies, allow slightly larger tolerance due to bin resolution
        // At 44.1kHz with 2048 frame size, bin width is ~21.5 Hz
        let tolerance = 30.0; // Slightly larger than one bin width
        assert!(
            (centroid - test_frequency).abs() < tolerance,
            "Expected centroid ~{} Hz, got {} Hz (tolerance: {} Hz)",
            test_frequency,
            centroid,
            tolerance
        );
    }

    #[test]
    fn test_spectral_centroid_1000hz() {
        let sample_rate = 44100u32;
        let test_frequency = 1000.0f32;

        let samples = generate_sine(test_frequency, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // 5% tolerance
        let tolerance = test_frequency * 0.05;
        assert!(
            (centroid - test_frequency).abs() < tolerance,
            "Expected centroid ~{} Hz, got {} Hz",
            test_frequency,
            centroid
        );
    }

    #[test]
    fn test_spectral_centroid_5000hz() {
        let sample_rate = 44100u32;
        let test_frequency = 5000.0f32;

        let samples = generate_sine(test_frequency, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // 5% tolerance
        let tolerance = test_frequency * 0.05;
        assert!(
            (centroid - test_frequency).abs() < tolerance,
            "Expected centroid ~{} Hz, got {} Hz",
            test_frequency,
            centroid
        );
    }

    #[test]
    fn test_spectral_centroid_with_harmonics() {
        let sample_rate = 44100u32;
        let fundamental = 440.0f32; // A4

        // Generate a complex tone with harmonics (like a real instrument)
        // Fundamental + 2nd harmonic (half amplitude) + 3rd harmonic (quarter amplitude)
        let harmonics = vec![(2, 0.5), (3, 0.25)];
        let samples = generate_harmonics(fundamental, &harmonics, sample_rate, DEFAULT_FRAME_SIZE);

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // With harmonics, centroid should be higher than the fundamental
        // The weighted average shifts upward due to higher harmonics
        assert!(
            centroid > fundamental,
            "Centroid ({} Hz) should be above fundamental ({} Hz) due to harmonics",
            centroid,
            fundamental
        );

        // But shouldn't be above the 3rd harmonic
        let third_harmonic = fundamental * 3.0;
        assert!(
            centroid < third_harmonic,
            "Centroid ({} Hz) should be below 3rd harmonic ({} Hz)",
            centroid,
            third_harmonic
        );
    }

    #[test]
    fn test_spectral_centroid_mixed_frequencies() {
        let sample_rate = 44100u32;

        // Generate two equal-amplitude sine waves at 500Hz and 2000Hz
        // The centroid should be somewhere in between
        let samples: Vec<f32> = (0..DEFAULT_FRAME_SIZE)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * 500.0 * t).sin() + (2.0 * PI * 2000.0 * t).sin()
            })
            .collect();

        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&samples);
        let centroid = analyzer.spectral_centroid(&spectrum);

        // The centroid should be between the two frequencies
        // With equal amplitudes, it should be approximately at the mean
        assert!(
            centroid > 500.0 && centroid < 2000.0,
            "Centroid ({} Hz) should be between 500 Hz and 2000 Hz",
            centroid
        );
    }

    #[test]
    fn test_spectral_centroid_empty_spectrum() {
        let analyzer = SpectralAnalyzer::new(44100);
        let empty_spectrum: Vec<f32> = vec![0.0; 1025];
        let centroid = analyzer.spectral_centroid(&empty_spectrum);

        assert_eq!(centroid, 0.0, "Empty spectrum should have centroid of 0");
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

    // ========================================================================
    // Comprehensive Spectral Flatness Tests
    // ========================================================================

    #[test]
    fn test_spectral_flatness_pure_tone_near_zero() {
        let sample_rate = 44100u32;

        // Test multiple pure tones - all should have very low flatness
        let frequencies = [100.0, 440.0, 1000.0, 5000.0, 10000.0];

        for freq in frequencies {
            let samples = generate_sine(freq, sample_rate, DEFAULT_FRAME_SIZE);
            let mut analyzer = SpectralAnalyzer::new(sample_rate);
            let spectrum = analyzer.compute_spectrum(&samples);
            let flatness = analyzer.spectral_flatness(&spectrum);

            assert!(
                flatness < 0.15,
                "Pure {} Hz tone should have low flatness (<0.15), got {}",
                freq,
                flatness
            );
        }
    }

    #[test]
    fn test_spectral_flatness_white_noise_high() {
        let sample_rate = 44100u32;

        // White noise should have high spectral flatness (near 1.0 for ideal white noise)
        // Real generated noise will be lower due to finite samples
        let noise = generate_noise(DEFAULT_FRAME_SIZE, 98765);
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let spectrum = analyzer.compute_spectrum(&noise);
        let flatness = analyzer.spectral_flatness(&spectrum);

        // White noise should have flatness > 0.3 (ideally closer to 1.0)
        assert!(
            flatness > 0.3,
            "White noise should have high flatness (>0.3), got {}",
            flatness
        );
        assert!(
            flatness <= 1.0,
            "Flatness should be capped at 1.0, got {}",
            flatness
        );
    }

    #[test]
    fn test_spectral_flatness_pink_noise() {
        let sample_rate = 44100u32;

        // Pink noise has 1/f spectrum - more energy in lower frequencies
        // Its flatness should be between pure tone and white noise
        let pink = generate_pink_noise(DEFAULT_FRAME_SIZE, 11111);
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let pink_spectrum = analyzer.compute_spectrum(&pink);
        let pink_flatness = analyzer.spectral_flatness(&pink_spectrum);

        // Pink noise flatness should be moderate
        assert!(
            pink_flatness > 0.1,
            "Pink noise flatness should be > 0.1, got {}",
            pink_flatness
        );
        assert!(
            pink_flatness < 1.0,
            "Pink noise flatness should be < 1.0, got {}",
            pink_flatness
        );
    }

    #[test]
    fn test_spectral_flatness_ordering() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Pure sine - most tonal
        let sine = generate_sine(440.0, sample_rate, DEFAULT_FRAME_SIZE);
        let sine_spectrum = analyzer.compute_spectrum(&sine);
        let sine_flatness = analyzer.spectral_flatness(&sine_spectrum);

        // Complex tone with harmonics - somewhat tonal
        let harmonics = vec![(2, 0.5), (3, 0.33), (4, 0.25), (5, 0.2)];
        let complex =
            generate_harmonics(440.0, &harmonics, sample_rate, DEFAULT_FRAME_SIZE);
        let complex_spectrum = analyzer.compute_spectrum(&complex);
        let complex_flatness = analyzer.spectral_flatness(&complex_spectrum);

        // Pink noise
        let pink = generate_pink_noise(DEFAULT_FRAME_SIZE, 22222);
        let pink_spectrum = analyzer.compute_spectrum(&pink);
        let pink_flatness = analyzer.spectral_flatness(&pink_spectrum);

        // White noise - most flat
        let white = generate_noise(DEFAULT_FRAME_SIZE, 33333);
        let white_spectrum = analyzer.compute_spectrum(&white);
        let white_flatness = analyzer.spectral_flatness(&white_spectrum);

        // Verify ordering: pure sine < complex tone < pink noise < white noise
        assert!(
            sine_flatness < complex_flatness,
            "Sine ({}) should be less flat than complex tone ({})",
            sine_flatness,
            complex_flatness
        );
        assert!(
            complex_flatness < pink_flatness,
            "Complex tone ({}) should be less flat than pink noise ({})",
            complex_flatness,
            pink_flatness
        );
        assert!(
            pink_flatness < white_flatness,
            "Pink noise ({}) should be less flat than white noise ({})",
            pink_flatness,
            white_flatness
        );
    }

    #[test]
    fn test_spectral_flatness_empty() {
        let analyzer = SpectralAnalyzer::new(44100);

        // Empty spectrum
        let empty: Vec<f32> = vec![];
        let flatness = analyzer.spectral_flatness(&empty);
        assert_eq!(flatness, 0.0, "Empty spectrum should have flatness 0");

        // All-zero spectrum
        let zeros: Vec<f32> = vec![0.0; 1025];
        let flatness = analyzer.spectral_flatness(&zeros);
        assert_eq!(flatness, 0.0, "All-zero spectrum should have flatness 0");
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

    // ========================================================================
    // Comprehensive Spectral Rolloff Tests
    // ========================================================================

    #[test]
    fn test_spectral_rolloff_pure_tones() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Test rolloff for various pure tones
        // For a pure sine, the rolloff should be at or very close to the sine frequency
        let test_frequencies = [200.0f32, 500.0, 1000.0, 3000.0, 8000.0];

        for freq in test_frequencies {
            let samples = generate_sine(freq, sample_rate, DEFAULT_FRAME_SIZE);
            let spectrum = analyzer.compute_spectrum(&samples);
            let rolloff = analyzer.spectral_rolloff(&spectrum, 0.85);

            // For pure tones, rolloff should be close to the frequency
            // Allow tolerance of ~200 Hz due to bin resolution
            let tolerance = 200.0;
            assert!(
                (rolloff - freq).abs() < tolerance,
                "Pure {} Hz tone should have rolloff ~{}, got {}",
                freq,
                freq,
                rolloff
            );
        }
    }

    #[test]
    fn test_spectral_rolloff_85_percent_threshold() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Generate a known spectrum with predictable energy distribution
        // Two equal-amplitude sines: 500 Hz and 2000 Hz
        let samples: Vec<f32> = (0..DEFAULT_FRAME_SIZE)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * 500.0 * t).sin() + (2.0 * PI * 2000.0 * t).sin()
            })
            .collect();

        let spectrum = analyzer.compute_spectrum(&samples);

        // At 85% threshold, rolloff should capture most energy up to around 2000 Hz
        let rolloff_85 = analyzer.spectral_rolloff(&spectrum, 0.85);
        let rolloff_95 = analyzer.spectral_rolloff(&spectrum, 0.95);

        // 95% rolloff should be >= 85% rolloff
        assert!(
            rolloff_95 >= rolloff_85,
            "95% rolloff ({}) should be >= 85% rolloff ({})",
            rolloff_95,
            rolloff_85
        );
    }

    #[test]
    fn test_spectral_rolloff_different_percentiles() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Use noise which has energy across all frequencies
        let noise = generate_noise(DEFAULT_FRAME_SIZE, 77777);
        let spectrum = analyzer.compute_spectrum(&noise);

        // Test various percentiles
        let rolloff_50 = analyzer.spectral_rolloff(&spectrum, 0.50);
        let rolloff_75 = analyzer.spectral_rolloff(&spectrum, 0.75);
        let rolloff_85 = analyzer.spectral_rolloff(&spectrum, 0.85);
        let rolloff_95 = analyzer.spectral_rolloff(&spectrum, 0.95);

        // Higher percentiles should give higher rolloff frequencies
        assert!(
            rolloff_50 <= rolloff_75,
            "50% rolloff ({}) should be <= 75% rolloff ({})",
            rolloff_50,
            rolloff_75
        );
        assert!(
            rolloff_75 <= rolloff_85,
            "75% rolloff ({}) should be <= 85% rolloff ({})",
            rolloff_75,
            rolloff_85
        );
        assert!(
            rolloff_85 <= rolloff_95,
            "85% rolloff ({}) should be <= 95% rolloff ({})",
            rolloff_85,
            rolloff_95
        );
    }

    #[test]
    fn test_spectral_rolloff_low_vs_high_frequency() {
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);

        // Very low frequency sine (200 Hz)
        let low_samples = generate_sine(200.0, sample_rate, DEFAULT_FRAME_SIZE);
        let low_spectrum = analyzer.compute_spectrum(&low_samples);
        let low_rolloff = analyzer.spectral_rolloff(&low_spectrum, 0.85);

        // Very high frequency sine (15 kHz)
        let high_samples = generate_sine(15000.0, sample_rate, DEFAULT_FRAME_SIZE);
        let high_spectrum = analyzer.compute_spectrum(&high_samples);
        let high_rolloff = analyzer.spectral_rolloff(&high_spectrum, 0.85);

        // Low frequency should have low rolloff, high frequency should have high rolloff
        assert!(
            low_rolloff < 1000.0,
            "Low frequency (200 Hz) should have rolloff < 1000 Hz, got {}",
            low_rolloff
        );
        assert!(
            high_rolloff > 10000.0,
            "High frequency (15 kHz) should have rolloff > 10000 Hz, got {}",
            high_rolloff
        );
    }

    #[test]
    fn test_spectral_rolloff_edge_cases() {
        let analyzer = SpectralAnalyzer::new(44100);

        // Empty spectrum
        let empty: Vec<f32> = vec![];
        let rolloff = analyzer.spectral_rolloff(&empty, 0.85);
        assert_eq!(rolloff, 0.0, "Empty spectrum should have rolloff 0");

        // All-zero spectrum
        let zeros: Vec<f32> = vec![0.0; 1025];
        let rolloff = analyzer.spectral_rolloff(&zeros, 0.85);
        assert_eq!(rolloff, 0.0, "All-zero spectrum should have rolloff 0");

        // Percentile clamping
        let sample_rate = 44100u32;
        let mut analyzer = SpectralAnalyzer::new(sample_rate);
        let samples = generate_sine(1000.0, sample_rate, DEFAULT_FRAME_SIZE);
        let spectrum = analyzer.compute_spectrum(&samples);

        // Percentile 0 should give very low rolloff
        let rolloff_0 = analyzer.spectral_rolloff(&spectrum, 0.0);
        assert!(
            rolloff_0 >= 0.0,
            "0% rolloff should be valid, got {}",
            rolloff_0
        );

        // Percentile 1.0 (100%) should return Nyquist or last bin
        let rolloff_100 = analyzer.spectral_rolloff(&spectrum, 1.0);
        assert!(
            rolloff_100 > 0.0,
            "100% rolloff should be > 0, got {}",
            rolloff_100
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

    // ========================================================================
    // Comprehensive Zero Crossing Rate Tests
    // ========================================================================

    #[test]
    fn test_zcr_theoretical_relationship() {
        // For a sine wave, ZCR ≈ 2 * frequency / sample_rate
        // This tests the theoretical relationship between frequency and ZCR
        let sample_rate = 44100u32;
        let duration_samples = 44100; // 1 second for good averaging

        let test_frequencies = [100.0f32, 200.0, 440.0, 1000.0, 2000.0, 5000.0];

        for freq in test_frequencies {
            let samples = generate_sine(freq, sample_rate, duration_samples);
            let zcr = zero_crossing_rate(&samples);

            // Theoretical ZCR = 2 * freq / sample_rate
            // (2 crossings per cycle, normalized by sample count)
            let theoretical_zcr = 2.0 * freq / sample_rate as f32;

            // Allow 5% tolerance
            let tolerance = theoretical_zcr * 0.05 + 0.001; // Add small constant for very low frequencies
            assert!(
                (zcr - theoretical_zcr).abs() < tolerance,
                "ZCR for {} Hz should be ~{:.6}, got {:.6} (diff: {:.6})",
                freq,
                theoretical_zcr,
                zcr,
                (zcr - theoretical_zcr).abs()
            );
        }
    }

    #[test]
    fn test_zcr_frequency_proportionality() {
        let sample_rate = 44100u32;
        let duration = 44100; // 1 second

        // ZCR should be proportional to frequency
        let freq_1 = 100.0;
        let freq_2 = 500.0; // 5x higher frequency
        let freq_3 = 1000.0; // 10x higher than freq_1

        let zcr_1 = zero_crossing_rate(&generate_sine(freq_1, sample_rate, duration));
        let zcr_2 = zero_crossing_rate(&generate_sine(freq_2, sample_rate, duration));
        let zcr_3 = zero_crossing_rate(&generate_sine(freq_3, sample_rate, duration));

        // Verify proportionality: zcr_2 ≈ 5 * zcr_1, zcr_3 ≈ 10 * zcr_1
        let ratio_2_1 = zcr_2 / zcr_1;
        let ratio_3_1 = zcr_3 / zcr_1;

        assert!(
            (ratio_2_1 - 5.0).abs() < 0.5,
            "ZCR ratio (500Hz/100Hz) should be ~5, got {}",
            ratio_2_1
        );
        assert!(
            (ratio_3_1 - 10.0).abs() < 1.0,
            "ZCR ratio (1000Hz/100Hz) should be ~10, got {}",
            ratio_3_1
        );
    }

    #[test]
    fn test_zcr_sine_vs_noise_vs_constant() {
        let sample_rate = 44100u32;
        let duration = 4410;

        // Constant signal (DC) - zero crossings
        let dc: Vec<f32> = vec![0.5; duration];
        let dc_zcr = zero_crossing_rate(&dc);
        assert_eq!(dc_zcr, 0.0, "DC signal should have ZCR of 0");

        // Low frequency sine - low ZCR
        let low_sine = generate_sine(100.0, sample_rate, duration);
        let low_zcr = zero_crossing_rate(&low_sine);

        // High frequency sine - higher ZCR
        let high_sine = generate_sine(5000.0, sample_rate, duration);
        let high_zcr = zero_crossing_rate(&high_sine);

        // White noise - high ZCR (around 0.5 for ideal random)
        let noise = generate_noise(duration, 12121);
        let noise_zcr = zero_crossing_rate(&noise);

        // Verify ordering
        assert!(
            dc_zcr < low_zcr,
            "DC ({}) should have lower ZCR than low freq sine ({})",
            dc_zcr,
            low_zcr
        );
        assert!(
            low_zcr < high_zcr,
            "Low freq ({}) should have lower ZCR than high freq ({})",
            low_zcr,
            high_zcr
        );

        // Noise should have high ZCR (random sign changes)
        assert!(
            noise_zcr > 0.3,
            "Noise ZCR should be high (>0.3), got {}",
            noise_zcr
        );
    }

    #[test]
    fn test_zcr_square_wave() {
        // A square wave has exactly 2 crossings per period
        let sample_rate = 44100u32;
        let frequency = 100.0;
        let duration = 44100; // 1 second = 100 periods

        // Generate square wave
        let samples: Vec<f32> = (0..duration)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let phase = (t * frequency).fract();
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();

        let zcr = zero_crossing_rate(&samples);

        // Expected: 2 crossings per period * 100 periods / 44100 samples ≈ 0.00453
        let expected_zcr = 2.0 * frequency / sample_rate as f32;
        let tolerance = expected_zcr * 0.1; // 10% tolerance for edge effects

        assert!(
            (zcr - expected_zcr).abs() < tolerance,
            "Square wave ZCR should be ~{:.6}, got {:.6}",
            expected_zcr,
            zcr
        );
    }

    #[test]
    fn test_zcr_with_dc_offset() {
        let sample_rate = 44100u32;
        let duration = 4410;

        // Sine wave with DC offset that doesn't cross zero
        let samples_no_cross: Vec<f32> = (0..duration)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // Amplitude 0.3 + DC offset 0.5 = always positive
                0.5 + 0.3 * (2.0 * PI * 440.0 * t).sin()
            })
            .collect();

        let zcr_no_cross = zero_crossing_rate(&samples_no_cross);
        assert_eq!(
            zcr_no_cross, 0.0,
            "Signal with DC offset that never crosses zero should have ZCR = 0"
        );

        // Sine wave with small DC offset that still crosses zero
        let samples_cross: Vec<f32> = (0..duration)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // Amplitude 1.0 + DC offset 0.3 = crosses zero
                0.3 + 1.0 * (2.0 * PI * 440.0 * t).sin()
            })
            .collect();

        let zcr_cross = zero_crossing_rate(&samples_cross);
        assert!(
            zcr_cross > 0.0,
            "Signal with small DC offset should still have crossings"
        );
    }

    #[test]
    fn test_zcr_deterministic() {
        // ZCR should be consistent for the same input
        let samples = generate_sine(440.0, 44100, 4410);

        let zcr1 = zero_crossing_rate(&samples);
        let zcr2 = zero_crossing_rate(&samples);

        assert_eq!(zcr1, zcr2, "ZCR should be deterministic");
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

    // ========================================================================
    // Additional High-Level Feature Function Tests
    // ========================================================================

    #[test]
    fn test_valence_extreme_values() {
        // Test with extreme SpectralFeatures values
        let very_bright = SpectralFeatures {
            centroid_mean: 15000.0, // Way above 8kHz ceiling
            flatness_mean: 0.0,     // Pure tone (maximally tonal)
            ..Default::default()
        };
        let valence_bright = compute_valence(&very_bright, 44100);
        assert!(
            (0.0..=1.0).contains(&valence_bright),
            "Extreme bright should still be in range, got {}",
            valence_bright
        );

        let very_dark = SpectralFeatures {
            centroid_mean: 50.0,  // Way below 1kHz floor
            flatness_mean: 1.0,   // Pure noise (maximally flat)
            ..Default::default()
        };
        let valence_dark = compute_valence(&very_dark, 44100);
        assert!(
            (0.0..=1.0).contains(&valence_dark),
            "Extreme dark should still be in range, got {}",
            valence_dark
        );

        // Verify ordering
        assert!(
            valence_bright > valence_dark,
            "Bright ({}) should have higher valence than dark ({})",
            valence_bright,
            valence_dark
        );
    }

    #[test]
    fn test_valence_tonality_contribution() {
        // Test that tonality (inverse flatness) contributes to valence
        let tonal = SpectralFeatures {
            centroid_mean: 4500.0, // Mid-range centroid
            flatness_mean: 0.0,    // Pure tone
            ..Default::default()
        };
        let noisy = SpectralFeatures {
            centroid_mean: 4500.0, // Same centroid
            flatness_mean: 1.0,    // Pure noise
            ..Default::default()
        };

        let valence_tonal = compute_valence(&tonal, 44100);
        let valence_noisy = compute_valence(&noisy, 44100);

        assert!(
            valence_tonal > valence_noisy,
            "Tonal ({}) should have higher valence than noisy ({}) at same centroid",
            valence_tonal,
            valence_noisy
        );
    }

    #[test]
    fn test_acousticness_all_components() {
        // Test each component of acousticness independently

        // High ZCR alone should reduce acousticness
        let high_zcr_only = SpectralFeatures {
            zcr_mean: 0.5,
            hf_energy_ratio: 0.0,
            spectral_flux_mean: 0.0,
            ..Default::default()
        };
        let acousticness_high_zcr = compute_acousticness(&high_zcr_only);

        // High HF alone should reduce acousticness
        let high_hf_only = SpectralFeatures {
            zcr_mean: 0.0,
            hf_energy_ratio: 0.9,
            spectral_flux_mean: 0.0,
            ..Default::default()
        };
        let acousticness_high_hf = compute_acousticness(&high_hf_only);

        // High flux alone should reduce acousticness
        let high_flux_only = SpectralFeatures {
            zcr_mean: 0.0,
            hf_energy_ratio: 0.0,
            spectral_flux_mean: 0.8,
            ..Default::default()
        };
        let acousticness_high_flux = compute_acousticness(&high_flux_only);

        // All low should give high acousticness
        let all_low = SpectralFeatures {
            zcr_mean: 0.0,
            hf_energy_ratio: 0.0,
            spectral_flux_mean: 0.0,
            ..Default::default()
        };
        let acousticness_low = compute_acousticness(&all_low);

        assert!(
            acousticness_low > acousticness_high_zcr,
            "All low ({}) should be more acoustic than high ZCR only ({})",
            acousticness_low,
            acousticness_high_zcr
        );
        assert!(
            acousticness_low > acousticness_high_hf,
            "All low ({}) should be more acoustic than high HF only ({})",
            acousticness_low,
            acousticness_high_hf
        );
        assert!(
            acousticness_low > acousticness_high_flux,
            "All low ({}) should be more acoustic than high flux only ({})",
            acousticness_low,
            acousticness_high_flux
        );
    }

    #[test]
    fn test_instrumentalness_boundary_values() {
        // Test exact boundary at vocal_band_energy = 1.0
        let boundary = SpectralFeatures {
            vocal_band_energy: 1.0,
            ..Default::default()
        };
        let instrumentalness_boundary = compute_instrumentalness(&boundary);
        assert_eq!(
            instrumentalness_boundary, 0.0,
            "At vocal_band_energy=1.0, instrumentalness should be 0.0, got {}",
            instrumentalness_boundary
        );

        // Test at 0.0
        let zero_vocal = SpectralFeatures {
            vocal_band_energy: 0.0,
            ..Default::default()
        };
        let instrumentalness_zero = compute_instrumentalness(&zero_vocal);
        assert_eq!(
            instrumentalness_zero, 1.0,
            "At vocal_band_energy=0.0, instrumentalness should be 1.0, got {}",
            instrumentalness_zero
        );

        // Test mid-point
        let mid = SpectralFeatures {
            vocal_band_energy: 0.5,
            ..Default::default()
        };
        let instrumentalness_mid = compute_instrumentalness(&mid);
        assert!(
            (instrumentalness_mid - 0.5).abs() < 0.01,
            "At vocal_band_energy=0.5, instrumentalness should be ~0.5, got {}",
            instrumentalness_mid
        );
    }

    #[test]
    fn test_speechiness_edge_cases() {
        // Test with zero values
        let all_zero = SpectralFeatures {
            zcr_std: 0.0,
            vocal_band_energy: 0.0,
            ..Default::default()
        };
        let speechiness_zero = compute_speechiness(&all_zero);
        assert!(
            (0.0..=1.0).contains(&speechiness_zero),
            "Zero inputs should still produce valid speechiness, got {}",
            speechiness_zero
        );

        // Test with high variance and moderate vocal energy (typical speech)
        let speech_like = SpectralFeatures {
            zcr_std: 0.15,          // High variance = speech-like modulation
            vocal_band_energy: 0.5, // Moderate vocal energy
            ..Default::default()
        };
        let speechiness_speech = compute_speechiness(&speech_like);

        // Test with low variance and high vocal energy (typical singing)
        let singing_like = SpectralFeatures {
            zcr_std: 0.02,          // Low variance = sustained tones
            vocal_band_energy: 0.7, // High vocal energy
            ..Default::default()
        };
        let speechiness_singing = compute_speechiness(&singing_like);

        assert!(
            speechiness_speech > speechiness_singing,
            "Speech-like ({}) should have higher speechiness than singing-like ({})",
            speechiness_speech,
            speechiness_singing
        );
    }

    #[test]
    fn test_all_features_default_spectral_features() {
        // Test all feature functions with default SpectralFeatures
        let default_features = SpectralFeatures::default();

        let valence = compute_valence(&default_features, 44100);
        let acousticness = compute_acousticness(&default_features);
        let instrumentalness = compute_instrumentalness(&default_features);
        let speechiness = compute_speechiness(&default_features);

        // All should be in valid range
        assert!(
            (0.0..=1.0).contains(&valence),
            "Default valence should be valid, got {}",
            valence
        );
        assert!(
            (0.0..=1.0).contains(&acousticness),
            "Default acousticness should be valid, got {}",
            acousticness
        );
        assert!(
            (0.0..=1.0).contains(&instrumentalness),
            "Default instrumentalness should be valid, got {}",
            instrumentalness
        );
        assert!(
            (0.0..=1.0).contains(&speechiness),
            "Default speechiness should be valid, got {}",
            speechiness
        );
    }

    #[test]
    fn test_features_with_synthetic_noise() {
        // End-to-end test with synthetic noise
        let sample_rate = 44100u32;
        let noise = generate_noise(sample_rate as usize, 54321);
        let features = analyze_spectral_features(&noise, sample_rate);

        let valence = compute_valence(&features, sample_rate);
        let acousticness = compute_acousticness(&features);
        let instrumentalness = compute_instrumentalness(&features);
        let speechiness = compute_speechiness(&features);

        // All should be in valid range
        assert!((0.0..=1.0).contains(&valence), "Noise valence: {}", valence);
        assert!(
            (0.0..=1.0).contains(&acousticness),
            "Noise acousticness: {}",
            acousticness
        );
        assert!(
            (0.0..=1.0).contains(&instrumentalness),
            "Noise instrumentalness: {}",
            instrumentalness
        );
        assert!(
            (0.0..=1.0).contains(&speechiness),
            "Noise speechiness: {}",
            speechiness
        );

        // Noise should have low acousticness (electronic/synthetic character)
        assert!(
            acousticness < 0.7,
            "White noise should have low-moderate acousticness, got {}",
            acousticness
        );
    }

    #[test]
    fn test_features_with_synthetic_complex_tone() {
        // End-to-end test with complex harmonic tone (like a musical instrument)
        let sample_rate = 44100u32;
        let harmonics = vec![(2, 0.7), (3, 0.5), (4, 0.3), (5, 0.2), (6, 0.15)];
        let complex = generate_harmonics(440.0, &harmonics, sample_rate, sample_rate as usize);
        let features = analyze_spectral_features(&complex, sample_rate);

        let valence = compute_valence(&features, sample_rate);
        let acousticness = compute_acousticness(&features);
        let instrumentalness = compute_instrumentalness(&features);
        let speechiness = compute_speechiness(&features);

        // All should be in valid range
        assert!(
            (0.0..=1.0).contains(&valence),
            "Complex tone valence: {}",
            valence
        );
        assert!(
            (0.0..=1.0).contains(&acousticness),
            "Complex tone acousticness: {}",
            acousticness
        );
        assert!(
            (0.0..=1.0).contains(&instrumentalness),
            "Complex tone instrumentalness: {}",
            instrumentalness
        );
        assert!(
            (0.0..=1.0).contains(&speechiness),
            "Complex tone speechiness: {}",
            speechiness
        );

        // A sustained harmonic tone should have low speechiness (no speech modulation)
        assert!(
            speechiness < 0.5,
            "Sustained harmonic tone should have low speechiness, got {}",
            speechiness
        );
    }

    #[test]
    fn test_spectral_analyzer_with_various_sample_rates() {
        // Test that analyzer works with different common sample rates
        let sample_rates = [22050u32, 44100, 48000, 96000];

        for rate in sample_rates {
            let analyzer = SpectralAnalyzer::new(rate);
            assert_eq!(analyzer.sample_rate(), rate);

            // Generate a 1kHz sine at this sample rate
            let samples = generate_sine(1000.0, rate, DEFAULT_FRAME_SIZE);
            let mut analyzer = SpectralAnalyzer::new(rate);
            let spectrum = analyzer.compute_spectrum(&samples);

            // Centroid should be close to 1kHz regardless of sample rate
            let centroid = analyzer.spectral_centroid(&spectrum);
            assert!(
                (centroid - 1000.0).abs() < 100.0,
                "At {} Hz sample rate, centroid should be ~1000 Hz, got {}",
                rate,
                centroid
            );
        }
    }

    #[test]
    fn test_spectral_analyzer_custom_frame_size() {
        let sample_rate = 44100u32;

        // Test with smaller frame size
        let small_frame = 1024;
        let analyzer_small = SpectralAnalyzer::with_params(sample_rate, small_frame, 256);
        assert_eq!(analyzer_small.frame_size(), small_frame);

        // Test with larger frame size
        let large_frame = 4096;
        let analyzer_large = SpectralAnalyzer::with_params(sample_rate, large_frame, 1024);
        assert_eq!(analyzer_large.frame_size(), large_frame);
    }
}
