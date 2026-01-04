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
}
