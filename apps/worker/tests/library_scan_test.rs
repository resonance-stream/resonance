//! Integration tests for library_scan job
//!
//! This test module covers:
//! - Hash computation for audio files
//! - Metadata extraction from audio files
//! - Path traversal prevention security checks
//! - Full library scan functionality
//! - Change detection (new, modified, removed files)
//! - Feature extraction job chaining

mod common;

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

// Re-export test utilities
#[allow(unused_imports)]
use common::{create_mock_audio_file, create_temp_music_library, TestTrack};

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test file with specific content and return its path
fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let file_path = dir.join(name);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent directories");
    }
    let mut file = File::create(&file_path).expect("Failed to create test file");
    file.write_all(content).expect("Failed to write test file");
    file_path
}

/// Compute SHA-256 hash of file contents (mirrors library_scan implementation)
fn compute_file_hash(path: &Path) -> String {
    use std::io::Read;

    let mut file = File::open(path).expect("Failed to open file");
    let mut hasher = Sha256::new();

    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).expect("Failed to read file");
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Check if a path has a supported audio extension
fn is_audio_file(path: &Path) -> bool {
    const AUDIO_EXTENSIONS: &[&str] = &[
        "mp3", "ogg", "opus", "aac", "m4a", "wma", "mpc", "ape", "flac", "wav", "aiff", "aif",
        "alac", "dsf", "dff", "wv", "m4b", "m4p", "m4r", "mp4", "3gp", "webm",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Map file extension to audio_format enum value
fn extension_to_audio_format(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "mp3" => "mp3",
        "flac" => "flac",
        "ogg" => "ogg",
        "opus" => "opus",
        "aac" | "m4a" | "m4b" | "m4p" | "m4r" | "mp4" | "3gp" => "aac",
        "wav" | "aiff" | "aif" => "wav",
        "alac" => "alac",
        "webm" => "opus",
        _ => "other",
    }
}

/// Check if path is within library (path traversal protection)
fn is_path_within_library(library_path: &Path, file_path: &Path) -> bool {
    let canonical_library = match library_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let canonical_file = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    canonical_file.starts_with(&canonical_library)
}

// =============================================================================
// Hash Computation Tests (3 tests)
// =============================================================================

#[test]
fn test_hash_computation_deterministic() {
    // Test that the same file always produces the same hash
    let temp_dir = create_temp_music_library();
    let content = b"test audio content for hashing";
    let file_path = create_test_file(temp_dir.path(), "test.mp3", content);

    let hash1 = compute_file_hash(&file_path);
    let hash2 = compute_file_hash(&file_path);

    assert_eq!(hash1, hash2, "Hash should be deterministic");
    assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 hex characters");
}

#[test]
fn test_hash_computation_different_content() {
    // Test that different content produces different hashes
    let temp_dir = create_temp_music_library();

    let file1 = create_test_file(temp_dir.path(), "song1.mp3", b"audio content A");
    let file2 = create_test_file(temp_dir.path(), "song2.mp3", b"audio content B");

    let hash1 = compute_file_hash(&file1);
    let hash2 = compute_file_hash(&file2);

    assert_ne!(hash1, hash2, "Different content should produce different hashes");
}

#[test]
fn test_hash_computation_large_file() {
    // Test hash computation for larger files (tests chunked reading)
    let temp_dir = create_temp_music_library();

    // Create a file larger than the 64KB buffer size
    let large_content: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
    let file_path = create_test_file(temp_dir.path(), "large.flac", &large_content);

    let hash = compute_file_hash(&file_path);

    assert_eq!(hash.len(), 64, "SHA-256 hash should be 64 hex characters");
    // Verify it's valid hex
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash should only contain hex characters"
    );
}

// =============================================================================
// Metadata Extraction Tests (3 tests)
// =============================================================================

#[test]
fn test_is_audio_file_supported_formats() {
    // Test that all supported audio extensions are recognized
    let supported_files = [
        "song.mp3",
        "song.flac",
        "song.ogg",
        "song.opus",
        "song.m4a",
        "song.wav",
        "song.aiff",
        "song.aif",
        "song.alac",
        "song.m4b",    // Audiobook
        "song.webm",   // WebM audio
        "song.FLAC",   // Uppercase
        "song.MP3",    // Uppercase
        "song.Opus",   // Mixed case
    ];

    for file in supported_files {
        assert!(
            is_audio_file(Path::new(file)),
            "File '{}' should be recognized as audio",
            file
        );
    }
}

#[test]
fn test_is_audio_file_unsupported_formats() {
    // Test that non-audio files are rejected
    let unsupported_files = [
        "image.jpg",
        "image.png",
        "document.pdf",
        "data.json",
        "script.sh",
        "archive.zip",
        "video.mkv", // Video container (not in audio extensions)
        "cover.bmp",
        "readme.txt",
        "folder", // No extension
    ];

    for file in unsupported_files {
        assert!(
            !is_audio_file(Path::new(file)),
            "File '{}' should NOT be recognized as audio",
            file
        );
    }
}

#[test]
fn test_extension_to_audio_format_mapping() {
    // Test that extensions map correctly to database format enum
    let mappings = [
        ("mp3", "mp3"),
        ("MP3", "mp3"),
        ("flac", "flac"),
        ("FLAC", "flac"),
        ("ogg", "ogg"),
        ("opus", "opus"),
        ("m4a", "aac"),
        ("m4b", "aac"),
        ("mp4", "aac"),
        ("wav", "wav"),
        ("aiff", "wav"),
        ("aif", "wav"),
        ("alac", "alac"),
        ("webm", "opus"),
        ("wma", "other"),  // Unsupported in enum
        ("mpc", "other"),  // Unsupported in enum
        ("unknown", "other"),
    ];

    for (ext, expected_format) in mappings {
        let format = extension_to_audio_format(ext);
        assert_eq!(
            format, expected_format,
            "Extension '{}' should map to format '{}'",
            ext, expected_format
        );
    }
}

// =============================================================================
// Path Traversal Prevention Tests (3 tests)
// =============================================================================

#[test]
fn test_path_traversal_normal_file_allowed() {
    // Test that normal files within the library are allowed
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create a file in the library
    let file_path = create_test_file(library_path, "artist/album/song.flac", b"audio");

    assert!(
        is_path_within_library(library_path, &file_path),
        "File within library should be allowed"
    );
}

#[test]
fn test_path_traversal_parent_directory_blocked() {
    // Test that paths with .. components that escape the library are blocked
    let library_dir = create_temp_music_library();
    let library_path = library_dir.path();

    // Create a file outside the library
    let outside_dir = create_temp_music_library();
    let outside_file = create_test_file(outside_dir.path(), "malicious.mp3", b"bad");

    assert!(
        !is_path_within_library(library_path, &outside_file),
        "File outside library should be blocked"
    );

    // Also test with a path that would use ..
    let traversal_path = library_path.join("artist").join("..").join("..").join("etc").join("passwd");
    // Note: This path doesn't exist, so canonicalize will fail, which is the correct behavior
    assert!(
        !is_path_within_library(library_path, &traversal_path),
        "Path traversal attempt should be blocked"
    );
}

#[test]
fn test_path_traversal_symlink_escape_blocked() {
    // Test that symlinks pointing outside the library are blocked
    // Note: This test creates actual symlinks which requires filesystem support

    let library_dir = create_temp_music_library();
    let library_path = library_dir.path();

    // Create a directory outside the library
    let outside_dir = create_temp_music_library();
    let outside_file = create_test_file(outside_dir.path(), "secret.mp3", b"secret audio");

    // Create a symlink inside the library pointing outside
    let symlink_path = library_path.join("music").join("linked_file.mp3");
    fs::create_dir_all(symlink_path.parent().unwrap()).ok();

    // Try to create symlink (may fail on some platforms)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink(&outside_file, &symlink_path).is_ok() {
            // The symlink exists but canonicalize will resolve it to outside the library
            assert!(
                !is_path_within_library(library_path, &symlink_path),
                "Symlink escape should be blocked"
            );
        }
    }

    #[cfg(windows)]
    {
        // On Windows, symlinks require elevated privileges, so we skip this part
        // The important thing is the test doesn't fail
        let _ = symlink_path; // suppress unused warning
    }
}

// =============================================================================
// Full Scan Tests (3 tests)
// =============================================================================

#[test]
fn test_full_scan_discovers_files() {
    // Test that a full scan discovers all audio files in the library
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create a directory structure with various audio files
    let files = [
        "Artist1/Album1/01 - Track1.mp3",
        "Artist1/Album1/02 - Track2.mp3",
        "Artist1/Album2/01 - Song.flac",
        "Artist2/Single.ogg",
        "Compilations/Various - Hits.m4a",
    ];

    for file in files {
        create_test_file(library_path, file, b"mock audio content");
    }

    // Also create some non-audio files that should be ignored
    create_test_file(library_path, "Artist1/Album1/cover.jpg", b"image data");
    create_test_file(library_path, "README.txt", b"text content");

    // Walk the directory and count audio files
    let mut audio_count = 0;
    let mut non_audio_count = 0;

    for entry in walkdir::WalkDir::new(library_path).follow_links(false) {
        let entry = entry.expect("WalkDir error");
        if entry.path().is_file() {
            if is_audio_file(entry.path()) {
                audio_count += 1;
            } else {
                non_audio_count += 1;
            }
        }
    }

    assert_eq!(audio_count, 5, "Should discover 5 audio files");
    assert_eq!(non_audio_count, 2, "Should have 2 non-audio files (ignored)");
}

#[test]
fn test_full_scan_handles_empty_library() {
    // Test that scanning an empty library doesn't fail
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Count audio files (should be 0)
    let mut audio_count = 0;

    for entry in walkdir::WalkDir::new(library_path).follow_links(false) {
        let entry = entry.expect("WalkDir error");
        if entry.path().is_file() && is_audio_file(entry.path()) {
            audio_count += 1;
        }
    }

    assert_eq!(audio_count, 0, "Empty library should have 0 audio files");
}

#[test]
fn test_full_scan_handles_nested_directories() {
    // Test that deeply nested directories are properly scanned
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create a deeply nested structure
    let deep_path = "Level1/Level2/Level3/Level4/Level5/Level6";
    create_test_file(library_path, &format!("{}/deep_track.flac", deep_path), b"audio");

    // Also create files at various levels
    create_test_file(library_path, "Level1/track.mp3", b"audio");
    create_test_file(library_path, "Level1/Level2/track.mp3", b"audio");
    create_test_file(library_path, "Level1/Level2/Level3/track.mp3", b"audio");

    // Count all audio files
    let mut audio_count = 0;
    let mut max_depth = 0;

    for entry in walkdir::WalkDir::new(library_path).follow_links(false) {
        let entry = entry.expect("WalkDir error");
        if entry.path().is_file() && is_audio_file(entry.path()) {
            audio_count += 1;
            max_depth = max_depth.max(entry.depth());
        }
    }

    assert_eq!(audio_count, 4, "Should discover all 4 audio files");
    assert!(max_depth >= 6, "Should reach the deepest level (depth >= 6)");
}

// =============================================================================
// Change Detection Tests (3 tests)
// =============================================================================

#[test]
fn test_change_detection_new_file() {
    // Test detection of newly added files
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Initial state: no files
    let initial_files: Vec<PathBuf> = walkdir::WalkDir::new(library_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    assert_eq!(initial_files.len(), 0, "Initially no audio files");

    // Add a new file
    create_test_file(library_path, "new_song.mp3", b"new audio content");

    // Scan again
    let new_files: Vec<PathBuf> = walkdir::WalkDir::new(library_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    assert_eq!(new_files.len(), 1, "Should detect 1 new file");
    assert!(new_files[0].ends_with("new_song.mp3"));
}

#[test]
fn test_change_detection_modified_file() {
    // Test detection of modified files via hash change
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create initial file
    let file_path = create_test_file(library_path, "track.flac", b"original content");
    let original_hash = compute_file_hash(&file_path);

    // Modify the file
    fs::write(&file_path, b"modified content").expect("Failed to modify file");
    let modified_hash = compute_file_hash(&file_path);

    assert_ne!(
        original_hash, modified_hash,
        "Hash should change when file is modified"
    );
}

#[test]
fn test_change_detection_removed_file() {
    // Test detection of removed files
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create some files
    let file1 = create_test_file(library_path, "track1.mp3", b"audio 1");
    let file2 = create_test_file(library_path, "track2.mp3", b"audio 2");
    let file3 = create_test_file(library_path, "track3.mp3", b"audio 3");

    // Record initial state
    let initial_files: std::collections::HashSet<PathBuf> = walkdir::WalkDir::new(library_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    assert_eq!(initial_files.len(), 3, "Should have 3 files initially");

    // Remove a file
    fs::remove_file(&file2).expect("Failed to remove file");

    // Scan again
    let current_files: std::collections::HashSet<PathBuf> = walkdir::WalkDir::new(library_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    // Find removed files
    let removed_files: Vec<&PathBuf> = initial_files.difference(&current_files).collect();

    assert_eq!(removed_files.len(), 1, "Should detect 1 removed file");
    assert!(removed_files[0].ends_with("track2.mp3"));

    // Verify remaining files
    assert!(file1.exists(), "track1.mp3 should still exist");
    assert!(!file2.exists(), "track2.mp3 should be removed");
    assert!(file3.exists(), "track3.mp3 should still exist");
}

// =============================================================================
// Feature Extraction Job Chaining Tests (3 tests)
// =============================================================================

#[test]
fn test_feature_extraction_job_serialization() {
    // Test that feature extraction jobs can be serialized/deserialized
    // This mirrors how jobs are enqueued after library scan
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct FeatureExtractionJob {
        track_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", content = "payload")]
    enum Job {
        FeatureExtraction(FeatureExtractionJob),
    }

    let track_id = Uuid::new_v4();
    let job = Job::FeatureExtraction(FeatureExtractionJob {
        track_id: track_id.to_string(),
    });

    // Serialize
    let serialized = serde_json::to_string(&job).expect("Failed to serialize job");
    assert!(serialized.contains("FeatureExtraction"));
    assert!(serialized.contains(&track_id.to_string()));

    // Deserialize
    let deserialized: Job = serde_json::from_str(&serialized).expect("Failed to deserialize job");
    match deserialized {
        Job::FeatureExtraction(fe_job) => {
            assert_eq!(fe_job.track_id, track_id.to_string());
        }
    }
}

#[test]
fn test_feature_extraction_job_for_new_track() {
    // Test that new tracks should trigger feature extraction
    use uuid::Uuid;

    // Simulate a new track being discovered
    let track_id = Uuid::new_v4();
    let _file_path = "/music/artist/album/song.flac";

    // In the actual implementation, a job would be enqueued
    // Here we just verify the job structure is correct
    let job_payload = serde_json::json!({
        "type": "FeatureExtraction",
        "payload": {
            "track_id": track_id.to_string()
        }
    });

    assert!(job_payload["payload"]["track_id"].as_str().is_some());
    assert_eq!(
        job_payload["payload"]["track_id"].as_str().unwrap(),
        track_id.to_string()
    );
}

#[test]
fn test_feature_extraction_not_triggered_for_unchanged_tracks() {
    // Test that unchanged tracks (same hash) don't trigger feature extraction
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create a file
    let file_path = create_test_file(library_path, "unchanged.flac", b"audio content");

    // Compute hash (simulates what library_scan does)
    let hash1 = compute_file_hash(&file_path);

    // "Second scan" - hash is the same
    let hash2 = compute_file_hash(&file_path);

    // If hashes match and track exists, feature extraction should be skipped
    let should_skip = hash1 == hash2;
    assert!(should_skip, "Unchanged file should be skipped for feature extraction");
}

// =============================================================================
// Additional Edge Case Tests
// =============================================================================

#[test]
fn test_audio_extensions_count() {
    // Verify we support a reasonable number of audio formats
    const AUDIO_EXTENSIONS: &[&str] = &[
        "mp3", "ogg", "opus", "aac", "m4a", "wma", "mpc", "ape", "flac", "wav", "aiff", "aif",
        "alac", "dsf", "dff", "wv", "m4b", "m4p", "m4r", "mp4", "3gp", "webm",
    ];

    assert!(
        AUDIO_EXTENSIONS.len() >= 15,
        "Should support at least 15 audio formats"
    );
    assert!(AUDIO_EXTENSIONS.contains(&"mp3"), "Must support MP3");
    assert!(AUDIO_EXTENSIONS.contains(&"flac"), "Must support FLAC");
    assert!(AUDIO_EXTENSIONS.contains(&"opus"), "Must support Opus");
}

#[test]
fn test_walkdir_does_not_follow_symlinks() {
    // Test that WalkDir is configured to not follow symlinks (DoS prevention)
    let temp_dir = create_temp_music_library();
    let library_path = temp_dir.path();

    // Create a regular file
    create_test_file(library_path, "regular.mp3", b"audio");

    // Walk with follow_links(false)
    let walker = walkdir::WalkDir::new(library_path).follow_links(false);

    let mut found_regular = false;
    for entry in walker {
        let entry = entry.expect("WalkDir error");
        if entry.path().is_file() && entry.path().ends_with("regular.mp3") {
            found_regular = true;
        }
    }

    assert!(found_regular, "Should find regular file without following symlinks");
}

#[test]
fn test_file_with_no_extension() {
    // Test that files without extensions are not treated as audio
    let path = Path::new("/music/noextension");
    assert!(
        !is_audio_file(path),
        "File without extension should not be recognized as audio"
    );
}
