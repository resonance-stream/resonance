//! K-means clustering for taste-based playlist generation
//!
//! This module provides clustering functionality for grouping user listening
//! history embeddings into distinct taste clusters. Each cluster represents
//! a cohesive musical preference that can be used to generate personalized
//! playlists.
//!
//! # Algorithm
//!
//! 1. Try k-means clustering with k=2,3,4 clusters
//! 2. Calculate silhouette score for each k
//! 3. Select the k with the highest silhouette score (if above threshold)
//! 4. Generate descriptive names for each cluster based on dominant mood/genre/energy
//!
//! # Example
//!
//! ```ignore
//! use resonance_worker::jobs::clustering::{cluster_user_taste, TasteCluster};
//!
//! let embeddings: Vec<(Uuid, Vec<f32>)> = fetch_user_embeddings().await?;
//! let clusters = cluster_user_taste(&embeddings);
//!
//! for cluster in clusters {
//!     println!("Cluster '{}' has {} tracks", cluster.suggested_name, cluster.track_ids.len());
//! }
//! ```

// Allow dead code since this module is consumed by the taste_clustered_playlist job
// which is implemented in a separate feature. All public API will be used.
#![allow(dead_code)]

use std::collections::HashMap;

use linfa::traits::{Fit, Predict};
use linfa::DatasetBase;
use linfa_clustering::KMeans;
use ndarray::Array2;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use uuid::Uuid;

// =============================================================================
// Configuration Constants
// =============================================================================

/// Minimum number of clusters to try
pub const MIN_CLUSTERS: usize = 2;

/// Maximum number of clusters to try
pub const MAX_CLUSTERS: usize = 4;

/// Minimum silhouette score threshold for accepting clustering results
/// Below this threshold, the data may not have distinct clusters
pub const MIN_SILHOUETTE_THRESHOLD: f64 = 0.2;

/// Minimum tracks per cluster for a cluster to be valid
pub const MIN_TRACKS_PER_CLUSTER: usize = 5;

/// Maximum iterations for k-means convergence
pub const KMEANS_MAX_ITERATIONS: u64 = 100;

/// Tolerance for k-means convergence
const KMEANS_TOLERANCE: f64 = 1e-4;

/// Random seed for reproducible clustering results
const KMEANS_RANDOM_SEED: u64 = 42;

/// Threshold for using sampling-based silhouette calculation
/// For datasets larger than this, we sample to avoid O(n²) computation
const SILHOUETTE_SAMPLE_THRESHOLD: usize = 500;

/// Number of samples to use for silhouette calculation on large datasets
/// This provides a good approximation while keeping computation reasonable
const SILHOUETTE_SAMPLE_SIZE: usize = 300;

// =============================================================================
// Types
// =============================================================================

/// A cluster of tracks representing a distinct taste preference
#[derive(Debug, Clone)]
pub struct TasteCluster {
    /// Cluster index (0-based)
    pub index: usize,

    /// Centroid vector in embedding space
    pub centroid: Vec<f32>,

    /// Track IDs belonging to this cluster
    pub track_ids: Vec<Uuid>,

    /// Dominant mood in this cluster (if available)
    pub dominant_mood: Option<String>,

    /// Dominant genre in this cluster (if available)
    pub dominant_genre: Option<String>,

    /// Average energy level (0.0 to 1.0)
    pub average_energy: f32,

    /// Average valence/positivity level (0.0 to 1.0)
    pub average_valence: f32,

    /// Human-readable cluster name (e.g., "Energetic Electronic")
    pub suggested_name: String,
}

/// Metadata for tracks used in cluster naming
#[derive(Debug, Clone, Default)]
pub struct TrackClusterMetadata {
    /// Track mood (e.g., "happy", "melancholic")
    pub mood: Option<String>,

    /// Track genre (e.g., "electronic", "indie")
    pub genre: Option<String>,

    /// Energy level (0.0 to 1.0)
    pub energy: f32,

    /// Valence/positivity level (0.0 to 1.0)
    pub valence: f32,
}

// =============================================================================
// Main Clustering Function
// =============================================================================

/// Cluster user taste based on track embeddings
///
/// Takes a collection of (track_id, embedding) pairs and groups them into
/// 2-4 clusters using k-means clustering. The optimal number of clusters
/// is selected based on silhouette score.
///
/// # Arguments
///
/// * `embeddings` - Slice of (track_id, embedding_vector) tuples
///
/// # Returns
///
/// A vector of `TasteCluster` structs, or an empty vector if:
/// - Not enough data points for clustering
/// - Silhouette score is below threshold
/// - Clustering fails for any reason
///
/// # Example
///
/// ```ignore
/// let embeddings = vec![
///     (Uuid::new_v4(), vec![0.1, 0.2, 0.3]),
///     (Uuid::new_v4(), vec![0.9, 0.8, 0.7]),
///     // ... more embeddings
/// ];
/// let clusters = cluster_user_taste(&embeddings);
/// ```
pub fn cluster_user_taste(embeddings: &[(Uuid, Vec<f32>)]) -> Vec<TasteCluster> {
    cluster_user_taste_with_metadata(embeddings, &HashMap::new())
}

/// Cluster user taste with track metadata for naming
///
/// Same as `cluster_user_taste` but accepts additional metadata for each track
/// to enable more descriptive cluster naming.
///
/// # Arguments
///
/// * `embeddings` - Slice of (track_id, embedding_vector) tuples
/// * `metadata` - Map of track_id to TrackClusterMetadata
///
/// # Returns
///
/// A vector of `TasteCluster` structs with populated metadata fields
pub fn cluster_user_taste_with_metadata(
    embeddings: &[(Uuid, Vec<f32>)],
    metadata: &HashMap<Uuid, TrackClusterMetadata>,
) -> Vec<TasteCluster> {
    // Check minimum data requirements
    let min_required = MIN_CLUSTERS * MIN_TRACKS_PER_CLUSTER;
    if embeddings.len() < min_required {
        tracing::debug!(
            embedding_count = embeddings.len(),
            min_required = min_required,
            "Not enough embeddings for clustering"
        );
        return Vec::new();
    }

    // Validate embedding dimensions are consistent
    if embeddings.is_empty() {
        return Vec::new();
    }

    let embedding_dim = embeddings[0].1.len();
    if embedding_dim == 0 {
        tracing::warn!("Empty embedding vectors provided");
        return Vec::new();
    }

    if !embeddings.iter().all(|(_, e)| e.len() == embedding_dim) {
        tracing::warn!("Inconsistent embedding dimensions");
        return Vec::new();
    }

    // Convert embeddings to ndarray format (f64 for numerical stability)
    let data: Array2<f64> = Array2::from_shape_fn((embeddings.len(), embedding_dim), |(i, j)| {
        embeddings[i].1[j] as f64
    });

    // Try different values of k and find the best
    let mut best_k = 0;
    let mut best_silhouette = f64::NEG_INFINITY;
    let mut best_labels: Option<Vec<usize>> = None;
    let mut best_centroids: Option<Array2<f64>> = None;

    for k in MIN_CLUSTERS..=MAX_CLUSTERS.min(embeddings.len() / MIN_TRACKS_PER_CLUSTER) {
        match run_kmeans(&data, k) {
            Ok((labels, centroids)) => {
                // Check that all clusters have minimum tracks
                let cluster_counts = count_cluster_sizes(&labels, k);
                if cluster_counts.iter().any(|&c| c < MIN_TRACKS_PER_CLUSTER) {
                    tracing::debug!(
                        k = k,
                        cluster_counts = ?cluster_counts,
                        "Skipping k={} due to small cluster",
                        k
                    );
                    continue;
                }

                let silhouette = calculate_silhouette_score(&data, &labels);

                tracing::debug!(
                    k = k,
                    silhouette = silhouette,
                    cluster_sizes = ?cluster_counts,
                    "K-means result"
                );

                if silhouette > best_silhouette {
                    best_silhouette = silhouette;
                    best_k = k;
                    best_labels = Some(labels);
                    best_centroids = Some(centroids);
                }
            }
            Err(e) => {
                tracing::debug!(k = k, error = %e, "K-means failed for k={}", k);
            }
        }
    }

    // Check if best clustering meets threshold
    if best_silhouette < MIN_SILHOUETTE_THRESHOLD {
        tracing::info!(
            best_silhouette = best_silhouette,
            threshold = MIN_SILHOUETTE_THRESHOLD,
            "Silhouette score below threshold, no distinct clusters found"
        );
        return Vec::new();
    }

    // Build cluster results
    let labels = match best_labels {
        Some(l) => l,
        None => return Vec::new(),
    };

    let centroids = match best_centroids {
        Some(c) => c,
        None => return Vec::new(),
    };

    tracing::info!(
        k = best_k,
        silhouette = best_silhouette,
        "Selected k={} clusters with silhouette={}",
        best_k,
        best_silhouette
    );

    build_clusters(embeddings, metadata, &labels, &centroids, best_k)
}

// =============================================================================
// K-Means Implementation
// =============================================================================

/// Run k-means clustering with the specified number of clusters
fn run_kmeans(data: &Array2<f64>, k: usize) -> Result<(Vec<usize>, Array2<f64>), String> {
    // Create dataset for linfa
    let dataset = DatasetBase::from(data.clone());

    // Create RNG with fixed seed for reproducibility
    let rng = Xoshiro256Plus::seed_from_u64(KMEANS_RANDOM_SEED);

    // Configure and run k-means
    let model = KMeans::params_with_rng(k, rng)
        .max_n_iterations(KMEANS_MAX_ITERATIONS)
        .tolerance(KMEANS_TOLERANCE)
        .fit(&dataset)
        .map_err(|e| format!("K-means fitting failed: {}", e))?;

    // Get cluster assignments by predicting on the same data
    let predictions = model.predict(dataset);
    let labels: Vec<usize> = predictions.targets().iter().copied().collect();

    // Get centroids
    let centroids = model.centroids().clone();

    Ok((labels, centroids))
}

/// Count the number of points in each cluster
fn count_cluster_sizes(labels: &[usize], k: usize) -> Vec<usize> {
    let mut counts = vec![0; k];
    for &label in labels {
        if label < k {
            counts[label] += 1;
        }
    }
    counts
}

// =============================================================================
// Silhouette Score Calculation
// =============================================================================

/// Calculate the silhouette score for a clustering result
///
/// The silhouette score measures how similar a point is to its own cluster
/// compared to other clusters. Values range from -1 to 1:
/// - +1: Points are well-matched to their cluster
/// - 0: Points are on the boundary between clusters
/// - -1: Points may be assigned to the wrong cluster
///
/// For large datasets (> 500 samples), this function uses sampling-based
/// calculation to avoid O(n²) complexity while still providing a good
/// approximation of the true silhouette score.
///
/// # Arguments
///
/// * `data` - The data points as a 2D array (n_samples x n_features)
/// * `labels` - Cluster assignment for each point
///
/// # Returns
///
/// The mean silhouette score across all points (or sampled points for large datasets)
pub fn calculate_silhouette_score(data: &Array2<f64>, labels: &[usize]) -> f64 {
    let n_samples = data.nrows();

    if n_samples < 2 {
        return 0.0;
    }

    // Get unique clusters
    let n_clusters = labels.iter().max().map_or(0, |&m| m + 1);
    if n_clusters < 2 {
        return 0.0;
    }

    // For large datasets, use sampling to avoid O(n²) computation
    if n_samples > SILHOUETTE_SAMPLE_THRESHOLD {
        return calculate_silhouette_score_sampled(data, labels, n_clusters);
    }

    // Full calculation for smaller datasets
    calculate_silhouette_score_full(data, labels, n_clusters)
}

/// Calculate silhouette score using all data points (O(n²) complexity)
fn calculate_silhouette_score_full(
    data: &Array2<f64>,
    labels: &[usize],
    n_clusters: usize,
) -> f64 {
    let n_samples = data.nrows();

    // Calculate silhouette for each point
    let silhouettes: Vec<f64> = (0..n_samples)
        .map(|i| calculate_point_silhouette(data, labels, i, n_clusters))
        .collect();

    // Return mean silhouette
    silhouettes.iter().sum::<f64>() / n_samples as f64
}

/// Calculate silhouette score using stratified sampling for large datasets
///
/// This function samples points proportionally from each cluster to maintain
/// representative coverage while reducing computation time from O(n²) to O(sample²).
fn calculate_silhouette_score_sampled(
    data: &Array2<f64>,
    labels: &[usize],
    n_clusters: usize,
) -> f64 {
    let n_samples = data.nrows();
    let sample_size = SILHOUETTE_SAMPLE_SIZE.min(n_samples);

    // Group indices by cluster for stratified sampling
    let mut cluster_indices: Vec<Vec<usize>> = vec![Vec::new(); n_clusters];
    for (idx, &label) in labels.iter().enumerate() {
        if label < n_clusters {
            cluster_indices[label].push(idx);
        }
    }

    // Create RNG with fixed seed for reproducibility
    let mut rng = Xoshiro256Plus::seed_from_u64(KMEANS_RANDOM_SEED);

    // Stratified sampling: sample proportionally from each cluster
    let mut sampled_indices: Vec<usize> = Vec::with_capacity(sample_size);

    // Calculate how many samples to take from each cluster (proportional)
    let total_points: usize = cluster_indices.iter().map(|c| c.len()).sum();
    let mut remaining_samples = sample_size;

    for (cluster_idx, indices) in cluster_indices.iter_mut().enumerate() {
        if indices.is_empty() {
            continue;
        }

        // Calculate proportional sample count for this cluster
        // For the last cluster, use all remaining samples to handle rounding
        let cluster_sample_count = if cluster_idx == n_clusters - 1 {
            remaining_samples
        } else {
            let proportion = indices.len() as f64 / total_points as f64;
            let count = (proportion * sample_size as f64).round() as usize;
            count.min(remaining_samples).min(indices.len())
        };

        if cluster_sample_count == 0 {
            continue;
        }

        // Shuffle and take samples
        indices.shuffle(&mut rng);
        sampled_indices.extend(indices.iter().take(cluster_sample_count));
        remaining_samples = remaining_samples.saturating_sub(cluster_sample_count);
    }

    // If we didn't get enough samples (due to rounding), fill randomly
    if sampled_indices.len() < sample_size {
        let mut all_indices: Vec<usize> = (0..n_samples)
            .filter(|i| !sampled_indices.contains(i))
            .collect();
        all_indices.shuffle(&mut rng);
        let additional = sample_size - sampled_indices.len();
        sampled_indices.extend(all_indices.into_iter().take(additional));
    }

    tracing::debug!(
        n_samples = n_samples,
        sample_size = sampled_indices.len(),
        "Using sampling-based silhouette calculation"
    );

    // Calculate silhouette for sampled points only
    let silhouettes: Vec<f64> = sampled_indices
        .iter()
        .map(|&i| calculate_point_silhouette(data, labels, i, n_clusters))
        .collect();

    if silhouettes.is_empty() {
        return 0.0;
    }

    silhouettes.iter().sum::<f64>() / silhouettes.len() as f64
}

/// Calculate silhouette score for a single point
fn calculate_point_silhouette(
    data: &Array2<f64>,
    labels: &[usize],
    point_idx: usize,
    n_clusters: usize,
) -> f64 {
    let point = data.row(point_idx);
    let point_cluster = labels[point_idx];

    // Calculate mean intra-cluster distance (a)
    let mut intra_distances = Vec::new();
    for (j, &label) in labels.iter().enumerate() {
        if j != point_idx && label == point_cluster {
            intra_distances.push(euclidean_distance(&point, &data.row(j)));
        }
    }

    // If point is alone in its cluster, silhouette is 0
    if intra_distances.is_empty() {
        return 0.0;
    }

    let a = intra_distances.iter().sum::<f64>() / intra_distances.len() as f64;

    // Calculate mean distance to nearest other cluster (b)
    let mut min_inter_distance = f64::INFINITY;

    for other_cluster in 0..n_clusters {
        if other_cluster == point_cluster {
            continue;
        }

        let mut inter_distances = Vec::new();
        for (j, &label) in labels.iter().enumerate() {
            if label == other_cluster {
                inter_distances.push(euclidean_distance(&point, &data.row(j)));
            }
        }

        if !inter_distances.is_empty() {
            let mean_dist = inter_distances.iter().sum::<f64>() / inter_distances.len() as f64;
            if mean_dist < min_inter_distance {
                min_inter_distance = mean_dist;
            }
        }
    }

    let b = min_inter_distance;

    // Silhouette coefficient
    if a.max(b) == 0.0 {
        0.0
    } else {
        (b - a) / a.max(b)
    }
}

/// Calculate Euclidean distance between two points
fn euclidean_distance(a: &ndarray::ArrayView1<f64>, b: &ndarray::ArrayView1<f64>) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

// =============================================================================
// Cluster Building and Naming
// =============================================================================

/// Build TasteCluster structs from clustering results
fn build_clusters(
    embeddings: &[(Uuid, Vec<f32>)],
    metadata: &HashMap<Uuid, TrackClusterMetadata>,
    labels: &[usize],
    centroids: &Array2<f64>,
    k: usize,
) -> Vec<TasteCluster> {
    let mut clusters = Vec::with_capacity(k);

    for cluster_idx in 0..k {
        // Collect track IDs for this cluster
        let track_ids: Vec<Uuid> = embeddings
            .iter()
            .zip(labels.iter())
            .filter(|(_, &label)| label == cluster_idx)
            .map(|((id, _), _)| *id)
            .collect();

        // Get centroid as Vec<f32>
        let centroid: Vec<f32> = centroids
            .row(cluster_idx)
            .iter()
            .map(|&v| v as f32)
            .collect();

        // Collect metadata for cluster tracks
        let cluster_metadata: Vec<&TrackClusterMetadata> = track_ids
            .iter()
            .filter_map(|id| metadata.get(id))
            .collect();

        // Extract moods and genres
        let moods: Vec<String> = cluster_metadata
            .iter()
            .filter_map(|m| m.mood.clone())
            .collect();

        let genres: Vec<String> = cluster_metadata
            .iter()
            .filter_map(|m| m.genre.clone())
            .collect();

        // Calculate average energy and valence
        let (avg_energy, avg_valence) = if cluster_metadata.is_empty() {
            (0.5, 0.5) // Default to neutral
        } else {
            let energy_sum: f32 = cluster_metadata.iter().map(|m| m.energy).sum();
            let valence_sum: f32 = cluster_metadata.iter().map(|m| m.valence).sum();
            let count = cluster_metadata.len() as f32;
            (energy_sum / count, valence_sum / count)
        };

        // Get dominant mood and genre
        let dominant_mood = find_dominant(&moods);
        let dominant_genre = find_dominant(&genres);

        // Generate cluster name
        let suggested_name =
            generate_cluster_name(&moods, &genres, avg_energy, avg_valence, cluster_idx);

        clusters.push(TasteCluster {
            index: cluster_idx,
            centroid,
            track_ids,
            dominant_mood,
            dominant_genre,
            average_energy: avg_energy,
            average_valence: avg_valence,
            suggested_name,
        });
    }

    clusters
}

// =============================================================================
// Naming Helpers
// =============================================================================

/// Find the most frequent item in a collection
///
/// Returns `None` if the collection is empty.
pub fn find_dominant(items: &[String]) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for item in items {
        *counts.entry(item.as_str()).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(item, _)| item.to_string())
}

/// Generate a human-readable cluster name based on characteristics
///
/// # Arguments
///
/// * `moods` - All moods in the cluster
/// * `genres` - All genres in the cluster
/// * `avg_energy` - Average energy level (0.0 to 1.0)
/// * `avg_valence` - Average valence level (0.0 to 1.0)
/// * `cluster_idx` - Cluster index (used as fallback)
///
/// # Returns
///
/// A descriptive name like "Energetic Electronic", "Chill Indie", or "Upbeat Pop"
pub fn generate_cluster_name(
    moods: &[String],
    genres: &[String],
    avg_energy: f32,
    avg_valence: f32,
    cluster_idx: usize,
) -> String {
    let dominant_mood = find_dominant(moods);
    let dominant_genre = find_dominant(genres);

    // Build name from components
    match (dominant_mood.as_ref(), dominant_genre.as_ref()) {
        (Some(mood), Some(genre)) => {
            // Capitalize first letters
            format!("{} {}", capitalize(mood), capitalize(genre))
        }
        (Some(mood), None) => {
            // Use energy descriptor + mood
            let energy_desc = energy_descriptor(avg_energy);
            format!("{} {}", capitalize(energy_desc), capitalize(mood))
        }
        (None, Some(genre)) => {
            // Use energy/valence descriptor + genre
            let descriptor = combined_descriptor(avg_energy, avg_valence);
            format!("{} {}", capitalize(descriptor), capitalize(genre))
        }
        (None, None) => {
            // Fallback to energy/valence description
            let descriptor = combined_descriptor(avg_energy, avg_valence);
            format!("{} Mix #{}", capitalize(descriptor), cluster_idx + 1)
        }
    }
}

/// Get an energy-based descriptor
pub fn energy_descriptor(energy: f32) -> &'static str {
    if energy < 0.35 {
        "chill"
    } else if energy < 0.65 {
        "moderate"
    } else {
        "energetic"
    }
}

/// Get a valence-based descriptor
pub fn valence_descriptor(valence: f32) -> &'static str {
    if valence < 0.35 {
        "melancholic"
    } else if valence < 0.65 {
        "neutral"
    } else {
        "uplifting"
    }
}

/// Get a combined energy/valence descriptor
fn combined_descriptor(energy: f32, valence: f32) -> &'static str {
    match (energy < 0.5, valence < 0.5) {
        (true, true) => "mellow",
        (true, false) => "peaceful",
        (false, true) => "intense",
        (false, false) => "upbeat",
    }
}

/// Capitalize the first letter of a string
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Helper Functions for Tests
    // =========================================================================

    fn create_synthetic_data(
        cluster_centers: &[Vec<f64>],
        points_per_cluster: usize,
        spread: f64,
    ) -> Vec<(Uuid, Vec<f32>)> {
        use std::f64::consts::PI;

        let mut data = Vec::new();
        let dim = cluster_centers[0].len();

        for (cluster_idx, center) in cluster_centers.iter().enumerate() {
            for point_idx in 0..points_per_cluster {
                // Generate points in a circle around the center
                let angle = 2.0 * PI * (point_idx as f64) / (points_per_cluster as f64);

                let mut point = vec![0.0f32; dim];
                for (d, val) in point.iter_mut().enumerate().take(dim) {
                    // Deterministic spread using angle and dimension
                    let offset = spread * (angle + d as f64).sin();
                    *val = (center[d] + offset) as f32;
                }

                data.push((
                    Uuid::from_u128((cluster_idx * 1000 + point_idx) as u128),
                    point,
                ));
            }
        }

        data
    }

    // =========================================================================
    // Silhouette Score Tests
    // =========================================================================

    #[test]
    fn test_silhouette_score_well_separated_clusters() {
        // Two well-separated clusters
        let data = Array2::from_shape_vec(
            (6, 2),
            vec![
                0.0, 0.0, 0.1, 0.1, -0.1, 0.1, // Cluster 0 around origin
                10.0, 10.0, 10.1, 10.1, 9.9, 10.1, // Cluster 1 far away
            ],
        )
        .unwrap();

        let labels = vec![0, 0, 0, 1, 1, 1];

        let score = calculate_silhouette_score(&data, &labels);

        // Well-separated clusters should have high silhouette score (> 0.7)
        assert!(
            score > 0.7,
            "Expected high silhouette for well-separated clusters, got {}",
            score
        );
    }

    #[test]
    fn test_silhouette_score_overlapping_clusters() {
        // Two overlapping clusters
        let data = Array2::from_shape_vec(
            (6, 2),
            vec![
                0.0, 0.0, 0.5, 0.5, 1.0, 1.0, // Cluster 0
                0.5, 0.5, 1.0, 1.0, 1.5, 1.5, // Cluster 1 (overlapping)
            ],
        )
        .unwrap();

        let labels = vec![0, 0, 0, 1, 1, 1];

        let score = calculate_silhouette_score(&data, &labels);

        // Overlapping clusters should have lower silhouette score
        assert!(
            score < 0.7,
            "Expected lower silhouette for overlapping clusters, got {}",
            score
        );
    }

    #[test]
    fn test_silhouette_score_single_cluster() {
        let data = Array2::from_shape_vec((4, 2), vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0])
            .unwrap();

        let labels = vec![0, 0, 0, 0];

        let score = calculate_silhouette_score(&data, &labels);

        // Single cluster should return 0
        assert!(
            (score - 0.0).abs() < 1e-10,
            "Expected 0 for single cluster, got {}",
            score
        );
    }

    #[test]
    fn test_silhouette_score_empty_data() {
        let data = Array2::from_shape_vec((0, 2), vec![]).unwrap();
        let labels: Vec<usize> = vec![];

        let score = calculate_silhouette_score(&data, &labels);
        assert!((score - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_silhouette_score_single_point() {
        let data = Array2::from_shape_vec((1, 2), vec![0.0, 0.0]).unwrap();
        let labels = vec![0];

        let score = calculate_silhouette_score(&data, &labels);
        assert!((score - 0.0).abs() < 1e-10);
    }

    // =========================================================================
    // Clustering Tests
    // =========================================================================

    #[test]
    fn test_cluster_user_taste_insufficient_data() {
        // Only 8 points, need at least MIN_CLUSTERS * MIN_TRACKS_PER_CLUSTER = 10
        let embeddings: Vec<(Uuid, Vec<f32>)> = (0..8)
            .map(|i| (Uuid::from_u128(i as u128), vec![i as f32; 3]))
            .collect();

        let clusters = cluster_user_taste(&embeddings);

        assert!(
            clusters.is_empty(),
            "Should return empty for insufficient data"
        );
    }

    #[test]
    fn test_cluster_user_taste_empty_embeddings() {
        let clusters = cluster_user_taste(&[]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_cluster_user_taste_zero_dimension() {
        let embeddings: Vec<(Uuid, Vec<f32>)> =
            (0..20).map(|i| (Uuid::from_u128(i as u128), vec![])).collect();

        let clusters = cluster_user_taste(&embeddings);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_cluster_user_taste_inconsistent_dimensions() {
        let embeddings = vec![
            (Uuid::new_v4(), vec![1.0, 2.0, 3.0]),
            (Uuid::new_v4(), vec![1.0, 2.0]), // Different dimension
        ];

        let clusters = cluster_user_taste(&embeddings);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_cluster_user_taste_well_separated_clusters() {
        // Create two well-separated clusters
        let centers = vec![
            vec![0.0, 0.0, 0.0],
            vec![100.0, 100.0, 100.0], // Very far away
        ];

        let embeddings = create_synthetic_data(&centers, 10, 0.5);

        let clusters = cluster_user_taste(&embeddings);

        // Should find 2 clusters
        assert_eq!(clusters.len(), 2, "Should find 2 distinct clusters");

        // Each cluster should have approximately 10 tracks
        for cluster in &clusters {
            assert!(
                cluster.track_ids.len() >= 8,
                "Cluster {} has {} tracks, expected ~10",
                cluster.index,
                cluster.track_ids.len()
            );
        }
    }

    #[test]
    fn test_cluster_user_taste_all_identical_points() {
        // All points are identical - no distinct clusters
        let embeddings: Vec<(Uuid, Vec<f32>)> = (0..20)
            .map(|i| (Uuid::from_u128(i as u128), vec![1.0, 1.0, 1.0]))
            .collect();

        let clusters = cluster_user_taste(&embeddings);

        // Should return empty because silhouette will be 0 (below threshold)
        // or k-means will fail to find distinct clusters
        assert!(
            clusters.is_empty() || clusters.len() == 1,
            "Identical points should not form distinct clusters"
        );
    }

    #[test]
    fn test_cluster_with_metadata() {
        let centers = vec![vec![0.0, 0.0], vec![50.0, 50.0]];
        let embeddings = create_synthetic_data(&centers, 10, 0.5);

        let mut metadata = HashMap::new();
        for (i, (id, _)) in embeddings.iter().enumerate() {
            let is_first_cluster = i < 10;
            metadata.insert(
                *id,
                TrackClusterMetadata {
                    mood: Some(if is_first_cluster {
                        "happy".to_string()
                    } else {
                        "melancholic".to_string()
                    }),
                    genre: Some(if is_first_cluster {
                        "pop".to_string()
                    } else {
                        "indie".to_string()
                    }),
                    energy: if is_first_cluster { 0.8 } else { 0.3 },
                    valence: if is_first_cluster { 0.9 } else { 0.2 },
                },
            );
        }

        let clusters = cluster_user_taste_with_metadata(&embeddings, &metadata);

        // Should find 2 clusters
        assert_eq!(clusters.len(), 2);

        // Each cluster should have metadata populated
        for cluster in &clusters {
            assert!(
                cluster.dominant_mood.is_some(),
                "Cluster {} should have dominant mood",
                cluster.index
            );
            assert!(
                cluster.dominant_genre.is_some(),
                "Cluster {} should have dominant genre",
                cluster.index
            );
            assert!(!cluster.suggested_name.is_empty());
        }
    }

    // =========================================================================
    // Naming Function Tests
    // =========================================================================

    #[test]
    fn test_find_dominant_empty() {
        assert_eq!(find_dominant(&[]), None);
    }

    #[test]
    fn test_find_dominant_single_item() {
        let items = vec!["rock".to_string()];
        assert_eq!(find_dominant(&items), Some("rock".to_string()));
    }

    #[test]
    fn test_find_dominant_multiple_items() {
        let items = vec![
            "rock".to_string(),
            "pop".to_string(),
            "rock".to_string(),
            "jazz".to_string(),
            "rock".to_string(),
        ];
        assert_eq!(find_dominant(&items), Some("rock".to_string()));
    }

    #[test]
    fn test_find_dominant_tie() {
        // In case of tie, returns one of them (implementation detail)
        let items = vec!["rock".to_string(), "pop".to_string()];
        let result = find_dominant(&items);
        assert!(result == Some("rock".to_string()) || result == Some("pop".to_string()));
    }

    #[test]
    fn test_energy_descriptor() {
        assert_eq!(energy_descriptor(0.0), "chill");
        assert_eq!(energy_descriptor(0.3), "chill");
        assert_eq!(energy_descriptor(0.5), "moderate");
        assert_eq!(energy_descriptor(0.6), "moderate");
        assert_eq!(energy_descriptor(0.7), "energetic");
        assert_eq!(energy_descriptor(1.0), "energetic");
    }

    #[test]
    fn test_valence_descriptor() {
        assert_eq!(valence_descriptor(0.0), "melancholic");
        assert_eq!(valence_descriptor(0.3), "melancholic");
        assert_eq!(valence_descriptor(0.5), "neutral");
        assert_eq!(valence_descriptor(0.6), "neutral");
        assert_eq!(valence_descriptor(0.7), "uplifting");
        assert_eq!(valence_descriptor(1.0), "uplifting");
    }

    #[test]
    fn test_generate_cluster_name_with_mood_and_genre() {
        let moods = vec!["happy".to_string(), "happy".to_string()];
        let genres = vec!["electronic".to_string(), "electronic".to_string()];
        let name = generate_cluster_name(&moods, &genres, 0.7, 0.8, 0);
        assert_eq!(name, "Happy Electronic");
    }

    #[test]
    fn test_generate_cluster_name_mood_only() {
        let moods = vec!["melancholic".to_string()];
        let genres: Vec<String> = vec![];
        let name = generate_cluster_name(&moods, &genres, 0.3, 0.2, 0);
        assert_eq!(name, "Chill Melancholic");
    }

    #[test]
    fn test_generate_cluster_name_genre_only() {
        let moods: Vec<String> = vec![];
        let genres = vec!["indie".to_string()];
        let name = generate_cluster_name(&moods, &genres, 0.3, 0.3, 0);
        assert_eq!(name, "Mellow Indie");
    }

    #[test]
    fn test_generate_cluster_name_no_metadata() {
        let moods: Vec<String> = vec![];
        let genres: Vec<String> = vec![];
        let name = generate_cluster_name(&moods, &genres, 0.8, 0.8, 2);
        assert_eq!(name, "Upbeat Mix #3");
    }

    #[test]
    fn test_generate_cluster_name_various_energy_valence() {
        let moods: Vec<String> = vec![];
        let genres: Vec<String> = vec![];

        // Low energy, low valence
        assert_eq!(
            generate_cluster_name(&moods, &genres, 0.3, 0.3, 0),
            "Mellow Mix #1"
        );

        // Low energy, high valence
        assert_eq!(
            generate_cluster_name(&moods, &genres, 0.3, 0.7, 0),
            "Peaceful Mix #1"
        );

        // High energy, low valence
        assert_eq!(
            generate_cluster_name(&moods, &genres, 0.7, 0.3, 0),
            "Intense Mix #1"
        );

        // High energy, high valence
        assert_eq!(
            generate_cluster_name(&moods, &genres, 0.7, 0.7, 0),
            "Upbeat Mix #1"
        );
    }

    // =========================================================================
    // TasteCluster Struct Tests
    // =========================================================================

    #[test]
    fn test_taste_cluster_default_values() {
        let cluster = TasteCluster {
            index: 0,
            centroid: vec![0.0, 0.0, 0.0],
            track_ids: vec![Uuid::new_v4()],
            dominant_mood: None,
            dominant_genre: None,
            average_energy: 0.5,
            average_valence: 0.5,
            suggested_name: "Test Cluster".to_string(),
        };

        assert_eq!(cluster.index, 0);
        assert_eq!(cluster.track_ids.len(), 1);
        assert!(cluster.dominant_mood.is_none());
        assert!((cluster.average_energy - 0.5).abs() < f32::EPSILON);
    }

    // =========================================================================
    // Configuration Constants Tests
    // =========================================================================

    #[test]
    fn test_configuration_constants_valid() {
        assert!(MIN_CLUSTERS >= 2, "Must have at least 2 clusters minimum");
        assert!(
            MAX_CLUSTERS >= MIN_CLUSTERS,
            "Max clusters must be >= min clusters"
        );
        assert!(
            MIN_SILHOUETTE_THRESHOLD > -1.0 && MIN_SILHOUETTE_THRESHOLD < 1.0,
            "Silhouette threshold must be in valid range"
        );
        assert!(
            MIN_TRACKS_PER_CLUSTER >= 1,
            "Must have at least 1 track per cluster"
        );
        assert!(
            KMEANS_MAX_ITERATIONS >= 10,
            "Should have reasonable max iterations"
        );
    }

    #[test]
    fn test_silhouette_sampling_constants_valid() {
        assert!(
            SILHOUETTE_SAMPLE_THRESHOLD > 0,
            "Sample threshold must be positive"
        );
        assert!(
            SILHOUETTE_SAMPLE_SIZE > 0,
            "Sample size must be positive"
        );
        assert!(
            SILHOUETTE_SAMPLE_SIZE < SILHOUETTE_SAMPLE_THRESHOLD,
            "Sample size should be less than threshold"
        );
    }

    // =========================================================================
    // Sampling-based Silhouette Tests
    // =========================================================================

    #[test]
    fn test_silhouette_uses_full_calculation_below_threshold() {
        // Create dataset with exactly SILHOUETTE_SAMPLE_THRESHOLD points
        // This should use full calculation
        let n_points = SILHOUETTE_SAMPLE_THRESHOLD;
        let dim = 2;

        // Create two well-separated clusters
        let mut data_vec = Vec::with_capacity(n_points * dim);
        let mut labels = Vec::with_capacity(n_points);

        for i in 0..n_points {
            if i < n_points / 2 {
                // Cluster 0 around (0, 0)
                data_vec.push(0.1 * (i as f64));
                data_vec.push(0.1 * (i as f64));
                labels.push(0);
            } else {
                // Cluster 1 around (100, 100)
                data_vec.push(100.0 + 0.1 * (i as f64));
                data_vec.push(100.0 + 0.1 * (i as f64));
                labels.push(1);
            }
        }

        let data = Array2::from_shape_vec((n_points, dim), data_vec).unwrap();
        let score = calculate_silhouette_score(&data, &labels);

        // Should get a high score for well-separated clusters
        assert!(score > 0.5, "Expected high silhouette score, got {}", score);
    }

    #[test]
    fn test_silhouette_uses_sampling_above_threshold() {
        // Create dataset larger than SILHOUETTE_SAMPLE_THRESHOLD
        let n_points = SILHOUETTE_SAMPLE_THRESHOLD + 100;
        let dim = 2;

        // Create two well-separated clusters
        let mut data_vec = Vec::with_capacity(n_points * dim);
        let mut labels = Vec::with_capacity(n_points);

        for i in 0..n_points {
            if i < n_points / 2 {
                // Cluster 0 around (0, 0)
                data_vec.push(0.1 * (i as f64 % 10.0));
                data_vec.push(0.1 * (i as f64 % 10.0));
                labels.push(0);
            } else {
                // Cluster 1 around (100, 100)
                data_vec.push(100.0 + 0.1 * (i as f64 % 10.0));
                data_vec.push(100.0 + 0.1 * (i as f64 % 10.0));
                labels.push(1);
            }
        }

        let data = Array2::from_shape_vec((n_points, dim), data_vec).unwrap();
        let score = calculate_silhouette_score(&data, &labels);

        // Should still get a reasonable score with sampling
        // (may be slightly different from full calculation, but should be positive for well-separated clusters)
        assert!(score > 0.3, "Expected positive silhouette score with sampling, got {}", score);
    }

    #[test]
    fn test_sampled_silhouette_consistent_with_seed() {
        // Verify that sampling produces consistent results due to fixed seed
        let n_points = SILHOUETTE_SAMPLE_THRESHOLD + 200;
        let dim = 3;

        let mut data_vec = Vec::with_capacity(n_points * dim);
        let mut labels = Vec::with_capacity(n_points);

        for i in 0..n_points {
            let cluster = i % 3;
            let base = (cluster * 50) as f64;
            data_vec.push(base + (i as f64 % 5.0));
            data_vec.push(base + (i as f64 % 7.0));
            data_vec.push(base + (i as f64 % 3.0));
            labels.push(cluster);
        }

        let data = Array2::from_shape_vec((n_points, dim), data_vec).unwrap();

        // Run multiple times - should get same result due to fixed seed
        let score1 = calculate_silhouette_score(&data, &labels);
        let score2 = calculate_silhouette_score(&data, &labels);

        assert!(
            (score1 - score2).abs() < 1e-10,
            "Sampled silhouette should be deterministic, got {} and {}",
            score1,
            score2
        );
    }

    #[test]
    fn test_sampled_silhouette_handles_imbalanced_clusters() {
        // Test with one large cluster and one small cluster
        let n_points = SILHOUETTE_SAMPLE_THRESHOLD + 100;
        let dim = 2;

        let mut data_vec = Vec::with_capacity(n_points * dim);
        let mut labels = Vec::with_capacity(n_points);

        // 90% in cluster 0, 10% in cluster 1
        let cluster1_size = n_points / 10;
        let cluster0_size = n_points - cluster1_size;

        for i in 0..cluster0_size {
            data_vec.push(0.1 * (i as f64 % 10.0));
            data_vec.push(0.1 * (i as f64 % 10.0));
            labels.push(0);
        }

        for i in 0..cluster1_size {
            data_vec.push(100.0 + 0.1 * (i as f64 % 10.0));
            data_vec.push(100.0 + 0.1 * (i as f64 % 10.0));
            labels.push(1);
        }

        let data = Array2::from_shape_vec((n_points, dim), data_vec).unwrap();
        let score = calculate_silhouette_score(&data, &labels);

        // Should handle imbalanced clusters gracefully
        assert!(score > 0.0, "Expected positive silhouette score for imbalanced clusters, got {}", score);
    }
}
