//! Background job definitions and handlers
//!
//! This module contains scheduled tasks including:
//! - Library scanning and metadata updates
//! - Audio feature extraction
//! - AI embedding generation
//! - Weekly Discover playlist creation
//! - Smart prefetch for autoplay
//! - Lidarr integration sync

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::error::{WorkerError, WorkerResult};
use crate::AppState;

pub mod embedding_generation;
pub mod feature_extraction;
pub mod key_detection;
pub mod library_scan;
pub mod lidarr_sync;
pub mod mood_detection;
pub mod prefetch;
pub mod rhythm_analysis;
pub mod spectral;
pub mod weekly_playlist;

// Re-export audio analysis types and functions for external use.
// These will be consumed when feature_extraction.rs integrates the new modules.
#[allow(unused_imports)]
pub use spectral::{
    analyze_spectral_features, zero_crossing_rate, SpectralAnalyzer, SpectralFeatures,
    DEFAULT_FRAME_SIZE, DEFAULT_HOP_SIZE,
};

#[allow(unused_imports)]
pub use rhythm_analysis::{
    analyze as analyze_rhythm, calculate_danceability, RhythmAnalyzer, RhythmFeatures,
};

#[allow(unused_imports)]
pub use key_detection::{analyze as analyze_key, compute_chromagram, estimate_key, KeyResult};

/// Job types that can be processed by the worker
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Job {
    /// Scan library for new/modified tracks
    LibraryScan(library_scan::LibraryScanJob),

    /// Extract audio features from a track
    FeatureExtraction(feature_extraction::FeatureExtractionJob),

    /// Generate AI embeddings for a track
    EmbeddingGeneration(embedding_generation::EmbeddingGenerationJob),

    /// Detect mood and generate AI description for a track
    MoodDetection(mood_detection::MoodDetectionJob),

    /// Generate weekly personalized playlist
    WeeklyPlaylist(weekly_playlist::WeeklyPlaylistJob),

    /// Sync with Lidarr for new releases
    LidarrSync(lidarr_sync::LidarrSyncJob),

    /// Prefetch tracks for autoplay
    Prefetch(prefetch::PrefetchJob),
}

/// Redis queue keys
pub mod queue {
    pub const JOBS_PENDING: &str = "resonance:jobs:pending";
    pub const JOBS_PROCESSING: &str = "resonance:jobs:processing";
    pub const JOBS_FAILED: &str = "resonance:jobs:failed";
}

/// Job runner that processes background jobs from Redis queue
pub struct JobRunner {
    state: Arc<AppState>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl JobRunner {
    /// Create a new job runner
    pub fn new(state: Arc<AppState>, shutdown_rx: broadcast::Receiver<()>) -> Self {
        Self { state, shutdown_rx }
    }

    /// Run the job processing loop
    pub async fn run(mut self) -> WorkerResult<()> {
        let poll_interval = Duration::from_secs(self.state.config.poll_interval_secs);

        tracing::info!(
            "Starting job runner with {} second poll interval",
            self.state.config.poll_interval_secs
        );

        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    tracing::info!("Job runner received shutdown signal");
                    break;
                }
                _ = tokio::time::sleep(poll_interval) => {
                    if let Err(e) = self.process_pending_jobs().await {
                        tracing::error!("Error processing jobs: {}", e);
                    }
                }
            }
        }

        tracing::info!("Job runner stopped");
        Ok(())
    }

    /// Process pending jobs from the queue
    async fn process_pending_jobs(&self) -> WorkerResult<()> {
        let mut conn = self.state.redis.get_multiplexed_async_connection().await?;

        // Try to pop a job from the pending queue
        let job_data: Option<String> = redis::cmd("LPOP")
            .arg(queue::JOBS_PENDING)
            .query_async(&mut conn)
            .await?;

        if let Some(data) = job_data {
            // Move job to processing queue
            let _: i64 = redis::cmd("RPUSH")
                .arg(queue::JOBS_PROCESSING)
                .arg(&data)
                .query_async(&mut conn)
                .await?;

            // Parse and execute the job
            match serde_json::from_str::<Job>(&data) {
                Ok(job) => {
                    tracing::info!("Processing job: {:?}", job);

                    if let Err(e) = self.execute_job(&job).await {
                        // Log using the WorkerError's severity-aware logging
                        e.log();

                        // Move to failed queue
                        let _: i64 = redis::cmd("LREM")
                            .arg(queue::JOBS_PROCESSING)
                            .arg(1)
                            .arg(&data)
                            .query_async(&mut conn)
                            .await?;

                        let _: i64 = redis::cmd("RPUSH")
                            .arg(queue::JOBS_FAILED)
                            .arg(&data)
                            .query_async(&mut conn)
                            .await?;
                    } else {
                        // Remove from processing queue
                        let _: i64 = redis::cmd("LREM")
                            .arg(queue::JOBS_PROCESSING)
                            .arg(1)
                            .arg(&data)
                            .query_async(&mut conn)
                            .await?;

                        tracing::info!("Job completed successfully");
                    }
                }
                Err(e) => {
                    let worker_err = WorkerError::InvalidJobData(e.to_string());
                    worker_err.log();

                    // Move malformed job to failed queue
                    let _: i64 = redis::cmd("LREM")
                        .arg(queue::JOBS_PROCESSING)
                        .arg(1)
                        .arg(&data)
                        .query_async(&mut conn)
                        .await?;

                    let _: i64 = redis::cmd("RPUSH")
                        .arg(queue::JOBS_FAILED)
                        .arg(&data)
                        .query_async(&mut conn)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Execute a specific job
    async fn execute_job(&self, job: &Job) -> WorkerResult<()> {
        match job {
            Job::LibraryScan(payload) => library_scan::execute(&self.state, payload).await,
            Job::FeatureExtraction(payload) => {
                feature_extraction::execute(&self.state, payload).await
            }
            Job::EmbeddingGeneration(payload) => {
                embedding_generation::execute(&self.state, payload).await
            }
            Job::MoodDetection(payload) => mood_detection::execute(&self.state, payload).await,
            Job::WeeklyPlaylist(payload) => weekly_playlist::execute(&self.state, payload).await,
            Job::LidarrSync(payload) => lidarr_sync::execute(&self.state, payload).await,
            Job::Prefetch(payload) => prefetch::execute(&self.state, payload).await,
        }
    }
}

/// Helper to enqueue a job
#[allow(dead_code)]
pub async fn enqueue_job(redis: &redis::Client, job: &Job) -> WorkerResult<()> {
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let data = serde_json::to_string(job)?;

    let _: i64 = redis::cmd("RPUSH")
        .arg(queue::JOBS_PENDING)
        .arg(&data)
        .query_async(&mut conn)
        .await?;

    tracing::debug!("Enqueued job: {:?}", job);
    Ok(())
}
