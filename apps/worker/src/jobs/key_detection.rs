//! Musical key detection using the Krumhansl-Schmuckler algorithm
//!
//! Implements chromagram extraction and key estimation with Camelot notation output.

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

// MIDI pitch calculation constants
/// A4 (concert pitch) MIDI note number
const MIDI_A4: f32 = 69.0;
/// Semitones per octave for pitch class calculation
const SEMITONES_PER_OCTAVE: f32 = 12.0;
/// A4 reference frequency in Hz (concert pitch)
const A4_FREQUENCY: f32 = 440.0;

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

    // Pre-allocate FFT buffer outside loop to avoid repeated allocation
    let mut fft_input = vec![Complex::new(0.0f32, 0.0f32); window_size];

    // Process overlapping windows
    let mut offset = 0;
    while offset + window_size <= samples.len() {
        // Extract and window the current frame into pre-allocated buffer
        for (i, (&s, &w)) in samples[offset..offset + window_size]
            .iter()
            .zip(hann_window.iter())
            .enumerate()
        {
            fft_input[i] = Complex::new(s * w, 0.0);
        }

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
            // MIDI pitch = A4_midi + semitones_per_octave * log2(freq / A4_freq)
            let midi_pitch = MIDI_A4 + SEMITONES_PER_OCTAVE * (freq / A4_FREQUENCY).log2();

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
        let correlation =
            pearson_correlation(chromagram, &rotate_profile(&MAJOR_PROFILE, rotation));
        if correlation > best_correlation {
            best_correlation = correlation;
            best_pitch_class = rotation;
            best_is_major = true;
        }
    }

    // Try all 12 rotations for minor keys
    for rotation in 0..12 {
        let correlation =
            pearson_correlation(chromagram, &rotate_profile(&MINOR_PROFILE, rotation));
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

    /// Generate a pure sine wave at a given frequency
    ///
    /// # Arguments
    /// * `freq` - Frequency in Hz
    /// * `duration` - Duration in seconds
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Returns
    /// Vector of audio samples
    fn generate_sine(freq: f32, duration: f32, sample_rate: u32) -> Vec<f32> {
        let samples = (duration * sample_rate as f32) as usize;
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq * t).sin()
            })
            .collect()
    }

    /// Generate a sine wave with attack/release envelope
    fn generate_sine_with_envelope(freq: f32, duration: f32, sample_rate: u32) -> Vec<f32> {
        let samples = (duration * sample_rate as f32) as usize;
        let attack_samples = samples / 10;
        let release_samples = samples / 10;

        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let envelope = if i < attack_samples {
                    i as f32 / attack_samples as f32
                } else if i > samples - release_samples {
                    (samples - i) as f32 / release_samples as f32
                } else {
                    1.0
                };
                envelope * (2.0 * PI * freq * t).sin()
            })
            .collect()
    }

    /// Generate multiple sine waves mixed together (for chords)
    fn generate_chord(frequencies: &[f32], duration: f32, sample_rate: u32) -> Vec<f32> {
        let samples = (duration * sample_rate as f32) as usize;
        let amplitude = 1.0 / frequencies.len() as f32; // Normalize amplitude

        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                frequencies
                    .iter()
                    .map(|&freq| amplitude * (2.0 * PI * freq * t).sin())
                    .sum()
            })
            .collect()
    }

    /// Generate white noise using a simple LCG (linear congruential generator)
    fn generate_white_noise(duration: f32, sample_rate: u32, seed: u32) -> Vec<f32> {
        let samples = (duration * sample_rate as f32) as usize;
        let mut state = seed;
        (0..samples)
            .map(|_| {
                // Simple LCG for deterministic "random" noise
                state = state.wrapping_mul(1103515245).wrapping_add(12345);
                // Convert to float in range [-1.0, 1.0]
                ((state >> 16) as f32 / 32768.0) - 1.0
            })
            .collect()
    }

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
        let x = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let y = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let corr = pearson_correlation(&x, &y);
        assert!(
            (corr - 1.0).abs() < 0.001,
            "Perfect correlation should be 1.0, got {}",
            corr
        );

        // Test perfect negative correlation
        let y_neg = [
            12.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        ];
        let corr_neg = pearson_correlation(&x, &y_neg);
        assert!(
            (corr_neg + 1.0).abs() < 0.001,
            "Perfect negative correlation should be -1.0, got {}",
            corr_neg
        );
    }

    #[test]
    fn test_rotate_profile() {
        let profile = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];

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

    // ============================================================
    // NEW TESTS: Synthetic audio in known keys
    // ============================================================

    /// Test A natural minor scale detection
    ///
    /// Generates an A natural minor scale and verifies the key detector
    /// correctly identifies it as A minor.
    #[test]
    fn test_a_minor_scale() {
        let sample_rate = 44100u32;
        let duration_per_note = 0.5f32;

        // A natural minor scale frequencies (A4 to A5)
        // A - B - C - D - E - F - G - A
        let frequencies = [
            440.00, // A4
            493.88, // B4
            523.25, // C5
            587.33, // D5
            659.25, // E5
            698.46, // F5
            783.99, // G5
            880.00, // A5 (octave)
        ];

        // Generate scale with envelope to avoid clicks
        let mut samples = Vec::new();
        for freq in frequencies {
            samples.extend(generate_sine_with_envelope(
                freq,
                duration_per_note,
                sample_rate,
            ));
        }

        let result = analyze(&samples, sample_rate);

        // Should detect A minor
        assert_eq!(result.key, "A", "Expected key A, got {}", result.key);
        assert_eq!(
            result.mode, "minor",
            "Expected mode minor, got {}",
            result.mode
        );
        assert_eq!(
            result.camelot, "8A",
            "Expected Camelot 8A for A minor, got {}",
            result.camelot
        );
        assert!(
            result.confidence > 0.5,
            "Expected confidence > 0.5, got {}",
            result.confidence
        );
    }

    /// Test G major chord detection
    ///
    /// Generates a G major triad (G-B-D) played simultaneously
    /// and verifies detection of G major or closely related key.
    #[test]
    fn test_g_major_chord() {
        let sample_rate = 44100u32;
        let duration = 3.0f32; // Longer duration for better frequency resolution

        // G major triad: G4 - B4 - D5
        let chord_frequencies = [
            392.00, // G4
            493.88, // B4
            587.33, // D5
        ];

        let samples = generate_chord(&chord_frequencies, duration, sample_rate);
        let result = analyze(&samples, sample_rate);

        // G major triad should detect as G major
        // Note: Chord detection may sometimes detect related keys due to limited
        // harmonic content, so we check for G major or its relative minor (E minor)
        let is_g_major = result.key == "G" && result.mode == "major";
        let is_e_minor = result.key == "E" && result.mode == "minor"; // Relative minor

        assert!(
            is_g_major || is_e_minor,
            "Expected G major or E minor (relative), got {} {}",
            result.key,
            result.mode
        );

        // If it's G major, verify Camelot notation
        if is_g_major {
            assert_eq!(
                result.camelot, "9B",
                "Expected Camelot 9B for G major, got {}",
                result.camelot
            );
        }

        assert!(
            result.confidence > 0.4,
            "Expected reasonable confidence for chord, got {}",
            result.confidence
        );
    }

    /// Comprehensive Camelot wheel notation test
    ///
    /// Verifies the Camelot wheel mapping is correct for all major
    /// and minor keys.
    #[test]
    fn test_camelot_notation_comprehensive() {
        // Test specific mappings mentioned in requirements
        assert_eq!(CAMELOT_MAJOR[0], "8B", "C major should be 8B");
        assert_eq!(CAMELOT_MINOR[9], "8A", "A minor should be 8A");
        assert_eq!(CAMELOT_MAJOR[7], "9B", "G major should be 9B");

        // Test that C major detected audio returns correct Camelot
        let sample_rate = 44100u32;
        let samples = generate_c_major_scale(sample_rate, 0.5);
        let result = analyze(&samples, sample_rate);

        if result.key == "C" && result.mode == "major" {
            assert_eq!(
                result.camelot, "8B",
                "C major detection should return Camelot 8B"
            );
        }

        // Verify relative major/minor relationship in Camelot system
        // Relative keys share the same number but different letter (A/B)
        // C major (8B) <-> A minor (8A)
        let c_major_num: String = CAMELOT_MAJOR[0]
            .chars()
            .filter(|c| c.is_numeric())
            .collect();
        let a_minor_num: String = CAMELOT_MINOR[9]
            .chars()
            .filter(|c| c.is_numeric())
            .collect();
        assert_eq!(
            c_major_num, a_minor_num,
            "Relative major/minor should have same Camelot number"
        );

        // Verify all major keys end with 'B' and minor with 'A'
        for camelot in &CAMELOT_MAJOR {
            assert!(
                camelot.ends_with('B'),
                "Major key Camelot notation should end with B: {}",
                camelot
            );
        }
        for camelot in &CAMELOT_MINOR {
            assert!(
                camelot.ends_with('A'),
                "Minor key Camelot notation should end with A: {}",
                camelot
            );
        }
    }

    /// Test confidence levels: tonal content vs noise
    ///
    /// Verifies that tonal content (pure sine wave) produces higher
    /// confidence than white noise.
    ///
    /// Note: The Krumhansl-Schmuckler algorithm maps Pearson correlation (-1 to 1)
    /// to confidence (0 to 1) via the formula: (correlation + 1) / 2.
    /// White noise has roughly uniform spectral energy, which can still produce
    /// moderate correlations with key profiles. The key insight is that tonal
    /// content should have HIGHER confidence than noise due to stronger pitch
    /// class organization.
    #[test]
    fn test_confidence_tonal_vs_noise() {
        let sample_rate = 44100u32;
        let duration = 2.0f32;

        // Generate pure tonal content (A440)
        let tonal_samples = generate_sine(440.0, duration, sample_rate);
        let tonal_result = analyze(&tonal_samples, sample_rate);

        // Generate white noise (deterministic for reproducibility)
        let noise_samples = generate_white_noise(duration, sample_rate, 42);
        let noise_result = analyze(&noise_samples, sample_rate);

        // Primary assertion: Tonal content should have higher confidence than noise
        // This is the most important test - tonal content organizes energy
        // into specific pitch classes, leading to better correlation with key profiles
        assert!(
            tonal_result.confidence > noise_result.confidence,
            "Tonal content (confidence={:.3}) should have higher confidence than noise (confidence={:.3})",
            tonal_result.confidence,
            noise_result.confidence
        );

        // Tonal content should have reasonably high confidence (good pitch class organization)
        assert!(
            tonal_result.confidence > 0.6,
            "Tonal content should have confidence > 0.6, got {:.3}",
            tonal_result.confidence
        );

        // The confidence difference should be meaningful (at least 0.05)
        // This ensures tonal content is distinctly more confident than noise
        let confidence_difference = tonal_result.confidence - noise_result.confidence;
        assert!(
            confidence_difference > 0.05,
            "Confidence difference ({:.3}) should be > 0.05 to indicate meaningful tonal detection",
            confidence_difference
        );
    }

    /// Test detection of a C major scale (from original test, extended)
    ///
    /// Verifies the full C major scale is detected correctly with
    /// proper Camelot notation.
    #[test]
    fn test_c_major_scale() {
        let sample_rate = 44100u32;

        // C major scale frequencies (C4 to B4 as specified in requirements)
        let frequencies = [
            261.63, // C4
            293.66, // D4
            329.63, // E4
            349.23, // F4
            392.00, // G4
            440.00, // A4
            493.88, // B4
        ];

        // Generate 0.5 seconds of each note as specified
        let mut samples = Vec::new();
        for freq in frequencies {
            samples.extend(generate_sine_with_envelope(freq, 0.5, sample_rate));
        }

        let result = analyze(&samples, sample_rate);

        assert_eq!(result.key, "C", "Expected key C, got {}", result.key);
        assert_eq!(
            result.mode, "major",
            "Expected mode major, got {}",
            result.mode
        );
        assert_eq!(
            result.camelot, "8B",
            "C major should return Camelot 8B, got {}",
            result.camelot
        );
    }

    /// Test detection of A minor arpeggio
    ///
    /// Generates an A minor arpeggio (A-C-E) to test minor key detection
    /// with chord-like content.
    #[test]
    fn test_a_minor_arpeggio() {
        let sample_rate = 44100u32;
        let duration_per_note = 0.5f32;

        // A minor arpeggio: A - C - E - A (octave)
        let frequencies = [
            440.00, // A4
            523.25, // C5
            659.25, // E5
            880.00, // A5
        ];

        // Generate arpeggio
        let mut samples = Vec::new();
        for freq in frequencies {
            samples.extend(generate_sine_with_envelope(
                freq,
                duration_per_note,
                sample_rate,
            ));
        }

        let result = analyze(&samples, sample_rate);

        // Should detect A minor or C major (relative keys)
        let is_a_minor = result.key == "A" && result.mode == "minor";
        let is_c_major = result.key == "C" && result.mode == "major";

        assert!(
            is_a_minor || is_c_major,
            "Expected A minor or C major (relative), got {} {}",
            result.key,
            result.mode
        );

        // Verify confidence is reasonable
        assert!(
            result.confidence > 0.4,
            "Expected confidence > 0.4, got {}",
            result.confidence
        );
    }

    /// Test chromagram for single frequency
    ///
    /// Verifies the chromagram correctly identifies the pitch class
    /// of isolated frequencies.
    #[test]
    fn test_chromagram_single_frequencies() {
        let sample_rate = 44100u32;
        let duration = 1.0f32;

        // Test each pitch class with its corresponding frequency
        let test_cases = [
            (261.63, 0, "C"),   // C4
            (277.18, 1, "C#"),  // C#4
            (293.66, 2, "D"),   // D4
            (311.13, 3, "D#"),  // D#4
            (329.63, 4, "E"),   // E4
            (349.23, 5, "F"),   // F4
            (369.99, 6, "F#"),  // F#4
            (392.00, 7, "G"),   // G4
            (415.30, 8, "G#"),  // G#4
            (440.00, 9, "A"),   // A4
            (466.16, 10, "A#"), // A#4
            (493.88, 11, "B"),  // B4
        ];

        for (freq, expected_class, name) in test_cases {
            let samples = generate_sine(freq, duration, sample_rate);
            let chromagram = compute_chromagram(&samples, sample_rate);

            // Find the pitch class with maximum energy
            let max_class = chromagram
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap();

            assert_eq!(
                max_class, expected_class,
                "Frequency {} Hz ({}) should map to pitch class {}, got {}",
                freq, name, expected_class, max_class
            );
        }
    }

    /// Test that short audio still produces reasonable results
    #[test]
    fn test_short_audio_handling() {
        let sample_rate = 44100u32;

        // Very short audio (0.1 seconds)
        let short_samples = generate_sine(440.0, 0.1, sample_rate);
        let result = analyze(&short_samples, sample_rate);

        // Should still produce a result (even if confidence is lower)
        assert!(!result.key.is_empty(), "Should produce a key result");
        assert!(
            result.mode == "major" || result.mode == "minor",
            "Mode should be major or minor"
        );
    }

    /// Test with multi-octave content
    #[test]
    fn test_multi_octave_detection() {
        let sample_rate = 44100u32;
        let duration = 2.0f32;

        // C notes across multiple octaves (should all contribute to C pitch class)
        let c_frequencies = [
            130.81,  // C3
            261.63,  // C4
            523.25,  // C5
            1046.50, // C6
        ];

        let samples = generate_chord(&c_frequencies, duration, sample_rate);
        let chromagram = compute_chromagram(&samples, sample_rate);

        // C (pitch class 0) should have the highest energy
        let max_class = chromagram
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        assert_eq!(
            max_class, 0,
            "Multi-octave C notes should produce peak at pitch class 0 (C), got {}",
            max_class
        );
    }
}
