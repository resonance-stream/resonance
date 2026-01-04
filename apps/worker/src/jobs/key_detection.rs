//! Musical key detection using the Krumhansl-Schmuckler algorithm
//!
//! Implements chromagram extraction and key estimation with Camelot notation output.

// Allow dead code for now - this module will be integrated into feature_extraction
#![allow(dead_code)]

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Krumhansl-Kessler major key profile weights
/// Correlations of pitch classes with major key perception
const MAJOR_PROFILE: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];

/// Krumhansl-Kessler minor key profile weights
/// Correlations of pitch classes with minor key perception
const MINOR_PROFILE: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// Standard pitch class names
const PITCH_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Camelot wheel notation for major keys
/// Index corresponds to pitch class (0=C, 1=C#, etc.)
const CAMELOT_MAJOR: [&str; 12] = [
    "8B",  // C major
    "3B",  // C# major
    "10B", // D major
    "5B",  // D# major
    "12B", // E major
    "7B",  // F major
    "2B",  // F# major
    "9B",  // G major
    "4B",  // G# major
    "11B", // A major
    "6B",  // A# major
    "1B",  // B major
];

/// Camelot wheel notation for minor keys
/// Index corresponds to pitch class (0=C, 1=C#, etc.)
const CAMELOT_MINOR: [&str; 12] = [
    "5A",  // C minor
    "12A", // C# minor
    "7A",  // D minor
    "2A",  // D# minor
    "9A",  // E minor
    "4A",  // F minor
    "11A", // F# minor
    "6A",  // G minor
    "1A",  // G# minor
    "8A",  // A minor
    "3A",  // A# minor
    "10A", // B minor
];

/// Result of key detection analysis
#[derive(Debug, Clone, PartialEq)]
pub struct KeyResult {
    /// Detected key (e.g., "C", "F#")
    pub key: String,
    /// Mode: "major" or "minor"
    pub mode: String,
    /// Confidence score (0.0-1.0), based on correlation strength
    pub confidence: f32,
    /// Camelot wheel notation (e.g., "8B", "11A")
    pub camelot: String,
}

/// Compute a chromagram (12-element pitch class energy distribution) from audio samples
///
/// The chromagram represents the distribution of energy across the 12 pitch classes
/// (C, C#, D, ..., B), regardless of octave.
///
/// # Arguments
/// * `samples` - Audio samples (mono, normalized to [-1.0, 1.0])
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// A 12-element array of normalized pitch class energies
pub fn compute_chromagram(samples: &[f32], sample_rate: u32) -> [f32; 12] {
    // Use a window size that captures enough frequency resolution
    // 4096 samples at 44100 Hz gives ~10.7 Hz resolution
    let window_size = 4096usize;
    let hop_size = window_size / 2;

    // Frequency range of interest: A0 (~27.5 Hz) to C8 (~4186 Hz)
    let min_freq = 27.5_f32;
    let max_freq = 4186.0_f32;

    let mut chromagram = [0.0f32; 12];
    let mut window_count = 0;

    // Create FFT planner and allocate buffers
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    // Create Hann window for smooth spectral analysis
    let hann_window: Vec<f32> = (0..window_size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos()))
        .collect();

    // Process overlapping windows
    let mut offset = 0;
    while offset + window_size <= samples.len() {
        // Extract and window the current frame
        let mut fft_input: Vec<Complex<f32>> = samples[offset..offset + window_size]
            .iter()
            .zip(hann_window.iter())
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        // Perform FFT
        fft.process(&mut fft_input);

        // Compute magnitude spectrum (only need positive frequencies)
        let freq_resolution = sample_rate as f32 / window_size as f32;

        // Map FFT bins to pitch classes
        for (bin, complex) in fft_input.iter().enumerate().take(window_size / 2) {
            let freq = bin as f32 * freq_resolution;

            // Skip frequencies outside our range of interest
            if freq < min_freq || freq > max_freq {
                continue;
            }

            // Convert frequency to MIDI pitch number
            // MIDI pitch = 69 + 12 * log2(freq / 440)
            let midi_pitch = 69.0 + 12.0 * (freq / 440.0).log2();

            // Get pitch class (0-11)
            let pitch_class = ((midi_pitch.round() as i32 % 12) + 12) % 12;

            // Accumulate energy (magnitude squared)
            let magnitude = complex.norm();
            chromagram[pitch_class as usize] += magnitude * magnitude;
        }

        window_count += 1;
        offset += hop_size;
    }

    // Normalize chromagram
    if window_count > 0 {
        for energy in &mut chromagram {
            *energy /= window_count as f32;
        }
    }

    // Normalize to sum to 1.0 (or leave as zeros if silent)
    let total: f32 = chromagram.iter().sum();
    if total > f32::EPSILON {
        for energy in &mut chromagram {
            *energy /= total;
        }
    }

    chromagram
}

/// Estimate the musical key from a chromagram using the Krumhansl-Schmuckler algorithm
///
/// Correlates the chromagram with all 24 possible key profiles (12 major + 12 minor)
/// and selects the key with the highest correlation.
///
/// # Arguments
/// * `chromagram` - 12-element pitch class energy distribution
///
/// # Returns
/// `KeyResult` with detected key, mode, confidence, and Camelot notation
pub fn estimate_key(chromagram: &[f32; 12]) -> KeyResult {
    let mut best_correlation: f32 = f32::NEG_INFINITY;
    let mut best_pitch_class = 0;
    let mut best_is_major = true;

    // Try all 12 rotations for major keys
    for rotation in 0..12 {
        let correlation = pearson_correlation(chromagram, &rotate_profile(&MAJOR_PROFILE, rotation));
        if correlation > best_correlation {
            best_correlation = correlation;
            best_pitch_class = rotation;
            best_is_major = true;
        }
    }

    // Try all 12 rotations for minor keys
    for rotation in 0..12 {
        let correlation = pearson_correlation(chromagram, &rotate_profile(&MINOR_PROFILE, rotation));
        if correlation > best_correlation {
            best_correlation = correlation;
            best_pitch_class = rotation;
            best_is_major = false;
        }
    }

    // Convert correlation to confidence (correlation ranges from -1 to 1)
    // Map to 0.0-1.0 range
    let confidence = ((best_correlation + 1.0) / 2.0).clamp(0.0, 1.0);

    let key = PITCH_NAMES[best_pitch_class].to_string();
    let mode = if best_is_major { "major" } else { "minor" }.to_string();
    let camelot = if best_is_major {
        CAMELOT_MAJOR[best_pitch_class]
    } else {
        CAMELOT_MINOR[best_pitch_class]
    }
    .to_string();

    KeyResult {
        key,
        mode,
        confidence,
        camelot,
    }
}

/// Rotate a key profile by the given number of semitones
///
/// This effectively transposes the profile to a different root note.
fn rotate_profile(profile: &[f32; 12], semitones: usize) -> [f32; 12] {
    let mut rotated = [0.0f32; 12];
    for (i, &value) in profile.iter().enumerate() {
        let new_index = (i + semitones) % 12;
        rotated[new_index] = value;
    }
    rotated
}

/// Calculate Pearson correlation coefficient between two arrays
///
/// Returns a value between -1.0 and 1.0 indicating the linear correlation.
fn pearson_correlation(x: &[f32; 12], y: &[f32; 12]) -> f32 {
    let n = 12.0f32;

    // Calculate means
    let mean_x: f32 = x.iter().sum::<f32>() / n;
    let mean_y: f32 = y.iter().sum::<f32>() / n;

    // Calculate covariance and standard deviations
    let mut covariance = 0.0f32;
    let mut var_x = 0.0f32;
    let mut var_y = 0.0f32;

    for i in 0..12 {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        covariance += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let std_x = var_x.sqrt();
    let std_y = var_y.sqrt();

    // Avoid division by zero
    if std_x < f32::EPSILON || std_y < f32::EPSILON {
        return 0.0;
    }

    covariance / (std_x * std_y)
}

/// Main entry point for key analysis
///
/// Combines chromagram extraction and key estimation into a single function.
///
/// # Arguments
/// * `samples` - Audio samples (mono, normalized to [-1.0, 1.0])
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// `KeyResult` with detected key, mode, confidence, and Camelot notation
pub fn analyze(samples: &[f32], sample_rate: u32) -> KeyResult {
    let chromagram = compute_chromagram(samples, sample_rate);
    estimate_key(&chromagram)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Generate a synthetic C major scale as audio samples
    fn generate_c_major_scale(sample_rate: u32, duration_per_note: f32) -> Vec<f32> {
        // C major scale frequencies (C4 to C5)
        let frequencies = [
            261.63, // C4
            293.66, // D4
            329.63, // E4
            349.23, // F4
            392.00, // G4
            440.00, // A4
            493.88, // B4
            523.25, // C5
        ];

        let samples_per_note = (sample_rate as f32 * duration_per_note) as usize;
        let mut samples = Vec::with_capacity(frequencies.len() * samples_per_note);

        for freq in frequencies {
            for i in 0..samples_per_note {
                let t = i as f32 / sample_rate as f32;
                // Generate a simple sine wave with envelope
                let envelope = if i < samples_per_note / 10 {
                    // Attack
                    i as f32 / (samples_per_note / 10) as f32
                } else if i > samples_per_note * 9 / 10 {
                    // Release
                    (samples_per_note - i) as f32 / (samples_per_note / 10) as f32
                } else {
                    1.0
                };
                let sample = envelope * (2.0 * PI * freq * t).sin();
                samples.push(sample);
            }
        }

        samples
    }

    #[test]
    fn test_c_major_scale_detection() {
        let sample_rate = 44100;
        let samples = generate_c_major_scale(sample_rate, 0.5);

        let result = analyze(&samples, sample_rate);

        // Should detect C major
        assert_eq!(result.key, "C", "Expected key C, got {}", result.key);
        assert_eq!(
            result.mode, "major",
            "Expected mode major, got {}",
            result.mode
        );
        assert_eq!(
            result.camelot, "8B",
            "Expected Camelot 8B, got {}",
            result.camelot
        );
        assert!(
            result.confidence > 0.5,
            "Expected confidence > 0.5, got {}",
            result.confidence
        );
    }

    #[test]
    fn test_chromagram_normalization() {
        // Test with silent input
        let silent_samples = vec![0.0f32; 44100];
        let chromagram = compute_chromagram(&silent_samples, 44100);

        // All values should be zero for silent input
        let total: f32 = chromagram.iter().sum();
        assert!(
            total < f32::EPSILON,
            "Silent chromagram should sum to ~0, got {}",
            total
        );
    }

    #[test]
    fn test_pearson_correlation() {
        // Test perfect positive correlation
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let corr = pearson_correlation(&x, &y);
        assert!(
            (corr - 1.0).abs() < 0.001,
            "Perfect correlation should be 1.0, got {}",
            corr
        );

        // Test perfect negative correlation
        let y_neg = [12.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        let corr_neg = pearson_correlation(&x, &y_neg);
        assert!(
            (corr_neg + 1.0).abs() < 0.001,
            "Perfect negative correlation should be -1.0, got {}",
            corr_neg
        );
    }

    #[test]
    fn test_rotate_profile() {
        let profile = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];

        // Rotate by 0 should give same array
        let rotated_0 = rotate_profile(&profile, 0);
        assert_eq!(rotated_0, profile);

        // Rotate by 1 should shift values
        let rotated_1 = rotate_profile(&profile, 1);
        assert_eq!(rotated_1[1], 1.0); // First element moved to index 1
        assert_eq!(rotated_1[0], 12.0); // Last element wrapped to index 0
    }

    #[test]
    fn test_camelot_notation() {
        // Test that Camelot notation is correctly mapped
        assert_eq!(CAMELOT_MAJOR[0], "8B"); // C major
        assert_eq!(CAMELOT_MINOR[9], "8A"); // A minor (relative minor of C major)

        // Verify the relationship: relative minor is 3 semitones below major
        // C major (8B) -> A minor (8A) - same number, different letter
    }

    #[test]
    fn test_key_result_structure() {
        let result = KeyResult {
            key: "A".to_string(),
            mode: "minor".to_string(),
            confidence: 0.85,
            camelot: "8A".to_string(),
        };

        assert_eq!(result.key, "A");
        assert_eq!(result.mode, "minor");
        assert!((result.confidence - 0.85).abs() < f32::EPSILON);
        assert_eq!(result.camelot, "8A");
    }

    /// Generate a pure A440 sine wave for testing
    #[test]
    fn test_a440_detection() {
        let sample_rate = 44100u32;
        let duration = 2.0f32;
        let frequency = 440.0f32; // A4

        let num_samples = (sample_rate as f32 * duration) as usize;
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * frequency * t).sin()
            })
            .collect();

        let chromagram = compute_chromagram(&samples, sample_rate);

        // A should have the highest energy (pitch class 9)
        let a_index = 9;
        let max_index = chromagram
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        assert_eq!(
            max_index, a_index,
            "A440 should show strongest energy in pitch class A (9), got {}",
            max_index
        );
    }
}
