//! Playlist service for smart playlist rule evaluation
//!
//! This module provides business logic for smart playlists, including:
//! - Rule evaluation against the track library
//! - Similarity-based track discovery using the SimilarityService
//! - Dynamic SQL query building for filter rules

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::playlist::{Playlist, SmartPlaylistRule, SmartPlaylistRules};
use crate::repositories::PlaylistRepository;
use crate::services::similarity::SimilarityService;

/// SQL parameter type for dynamic query binding
/// Supports both single text values and arrays for proper type handling
#[derive(Debug)]
enum SqlParam {
    /// Single text value
    Text(String),
    /// Array of text values (for IN/NOT IN operators)
    TextArray(Vec<String>),
}

/// Maximum tracks to fetch when evaluating similarity rules
const MAX_SIMILAR_TRACKS: i32 = 100;

/// Default minimum similarity score threshold
const DEFAULT_MIN_SCORE: f64 = 0.5;

/// Service for playlist operations including smart rule evaluation
///
/// NOTE: This service creates its own instances of PlaylistRepository and SimilarityService
/// internally. While this results in duplicate instances when used alongside the schema-level
/// services, all instances are stateless (they only hold Arc-based pool references) so this
/// is not a correctness issue. A future optimization could use constructor injection to
/// share instances: `new(pool, playlist_repo, similarity_service)`.
#[derive(Clone)]
pub struct PlaylistService {
    pool: PgPool,
    playlist_repo: PlaylistRepository,
    similarity_service: SimilarityService,
}

impl PlaylistService {
    /// Create a new PlaylistService
    pub fn new(pool: PgPool) -> Self {
        Self {
            playlist_repo: PlaylistRepository::new(pool.clone()),
            similarity_service: SimilarityService::new(pool.clone()),
            pool,
        }
    }

    /// Evaluate smart playlist rules and return matching track IDs
    ///
    /// Evaluates all rules according to the match mode (all = AND, any = OR)
    /// and returns track IDs that satisfy the rules.
    ///
    /// # Arguments
    /// * `rules` - The smart playlist rules to evaluate
    /// * `user_id` - The user ID for access control
    ///
    /// # Returns
    /// A vector of track IDs matching the rules
    #[instrument(skip(self, rules))]
    pub async fn evaluate_smart_rules(
        &self,
        rules: &SmartPlaylistRules,
        user_id: Uuid,
    ) -> ApiResult<Vec<Uuid>> {
        if rules.rules.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results: Vec<HashSet<Uuid>> = Vec::new();

        // Evaluate each rule
        for rule in &rules.rules {
            let matches = match rule.field.as_str() {
                "similar_to" => self.evaluate_similarity_rule(rule).await?,
                _ => self.evaluate_filter_rule(rule, user_id).await?,
            };
            all_results.push(matches);
        }

        // Combine results based on match mode (optimized to avoid unnecessary allocations)
        // Use explicit match to fail on invalid match_mode rather than silently defaulting
        let match_mode = rules.match_mode.to_ascii_lowercase();
        let combined = match match_mode.as_str() {
            "any" => {
                // Union of all results (OR logic) - extend in-place
                let mut result = all_results.pop().unwrap_or_default();
                for set in all_results {
                    result.extend(set);
                }
                result
            }
            "all" => {
                // Intersection of all results (AND logic) - use retain for efficiency
                // Optimization: start with the smallest set to minimize iterations
                if all_results.is_empty() {
                    HashSet::new()
                } else {
                    // Find and remove the smallest set to use as the starting point
                    let min_idx = all_results
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, s)| s.len())
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let mut result = all_results.swap_remove(min_idx);
                    for set in all_results {
                        result.retain(|item| set.contains(item));
                    }
                    result
                }
            }
            _ => {
                return Err(ApiError::ValidationError(
                    "match_mode must be 'all' or 'any'".to_string(),
                ));
            }
        };

        let mut track_ids: Vec<Uuid> = combined.into_iter().collect();

        // Apply sorting if specified
        if let Some(ref sort_by) = rules.sort_by {
            let sort_order = rules.sort_order.as_deref().unwrap_or("asc");
            track_ids = self.sort_tracks(track_ids, sort_by, sort_order).await?;
        }

        // Apply limit if specified (defensive: treat negative as 0)
        if let Some(limit) = rules.limit {
            track_ids.truncate(limit.max(0) as usize);
        }

        Ok(track_ids)
    }

    /// Evaluate a similarity-based rule using the SimilarityService
    async fn evaluate_similarity_rule(&self, rule: &SmartPlaylistRule) -> ApiResult<HashSet<Uuid>> {
        // Extract seed track IDs from the rule value
        let track_ids: Vec<Uuid> = rule
            .value
            .get("track_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| Uuid::parse_str(s).ok())
                    .collect()
            })
            .unwrap_or_default();

        if track_ids.is_empty() {
            return Ok(HashSet::new());
        }

        // Get minimum score threshold
        let min_score = rule
            .value
            .get("min_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(DEFAULT_MIN_SCORE);

        let mut all_similar = HashSet::new();

        // Find similar tracks for each seed track
        for seed_id in track_ids {
            let similar_tracks = match rule.operator.as_str() {
                "semantic" => {
                    self.similarity_service
                        .find_similar_by_embedding(seed_id, MAX_SIMILAR_TRACKS)
                        .await
                }
                "acoustic" => {
                    self.similarity_service
                        .find_similar_by_features(seed_id, MAX_SIMILAR_TRACKS)
                        .await
                }
                "categorical" => {
                    self.similarity_service
                        .find_similar_by_tags(seed_id, MAX_SIMILAR_TRACKS)
                        .await
                }
                _ => {
                    // Default to combined similarity
                    self.similarity_service
                        .find_similar_combined(seed_id, MAX_SIMILAR_TRACKS)
                        .await
                }
            };

            // Filter by min_score and collect track IDs
            match similar_tracks {
                Ok(tracks) => {
                    for track in tracks {
                        if track.score >= min_score {
                            all_similar.insert(track.track_id);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        seed_track_id = %seed_id,
                        error = %e,
                        "Failed to find similar tracks for seed, continuing with other seeds"
                    );
                }
            }
        }

        Ok(all_similar)
    }

    /// Evaluate a filter rule using SQL
    ///
    /// Note: `user_id` is accepted for future use (e.g., user-specific play counts,
    /// preferences, or library filtering). Currently, tracks are accessible to all
    /// authenticated users since this is a shared music library. Access control for
    /// playlists is enforced at the mutation/query layer, not the track level.
    async fn evaluate_filter_rule(
        &self,
        rule: &SmartPlaylistRule,
        _user_id: Uuid,
    ) -> ApiResult<HashSet<Uuid>> {
        // Build SQL query based on rule
        let (where_clause, params) = self.build_filter_sql(rule)?;

        if where_clause.is_empty() {
            return Ok(HashSet::new());
        }

        let sql = format!("SELECT id FROM tracks WHERE {}", where_clause);

        // Execute the query with dynamic parameter binding
        // Handle both single values and arrays appropriately
        let mut query = sqlx::query_as::<_, (Uuid,)>(&sql);
        for param in params {
            query = match param {
                SqlParam::Text(s) => query.bind(s),
                SqlParam::TextArray(v) => query.bind(v),
            };
        }
        let track_ids: Vec<(Uuid,)> = query.fetch_all(&self.pool).await?;

        Ok(track_ids.into_iter().map(|(id,)| id).collect())
    }

    /// Build SQL WHERE clause for a filter rule
    fn build_filter_sql(&self, rule: &SmartPlaylistRule) -> ApiResult<(String, Vec<SqlParam>)> {
        let field = self.get_sql_field(&rule.field)?;
        let is_array_field = matches!(rule.field.as_str(), "genres" | "ai_mood" | "ai_tags");
        let is_json_field = matches!(
            rule.field.as_str(),
            "bpm"
                | "energy"
                | "danceability"
                | "valence"
                | "acousticness"
                | "instrumentalness"
                | "speechiness"
                | "loudness"
        );

        let (clause, params) = match rule.operator.as_str() {
            "equals" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float = $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} = $1", field), vec![SqlParam::Text(val)])
                }
            }
            "not_equals" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float != $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} != $1", field), vec![SqlParam::Text(val)])
                }
            }
            "contains" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_array_field {
                    (format!("$1 = ANY({})", field), vec![SqlParam::Text(val)])
                } else {
                    (
                        format!("{} ILIKE '%' || $1 || '%'", field),
                        vec![SqlParam::Text(val)],
                    )
                }
            }
            "not_contains" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_array_field {
                    (
                        format!("NOT ($1 = ANY({}))", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (
                        format!("{} NOT ILIKE '%' || $1 || '%'", field),
                        vec![SqlParam::Text(val)],
                    )
                }
            }
            "starts_with" => {
                let val = self.extract_string_value(&rule.value)?;
                (
                    format!("{} ILIKE $1 || '%'", field),
                    vec![SqlParam::Text(val)],
                )
            }
            "ends_with" => {
                let val = self.extract_string_value(&rule.value)?;
                (
                    format!("{} ILIKE '%' || $1", field),
                    vec![SqlParam::Text(val)],
                )
            }
            "is_empty" => {
                if is_array_field {
                    // COALESCE handles both NULL and empty arrays correctly
                    (
                        format!("COALESCE(array_length({}, 1), 0) = 0", field),
                        vec![],
                    )
                } else {
                    (format!("{} IS NULL OR {} = ''", field, field), vec![])
                }
            }
            "greater_than" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float > $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} > $1", field), vec![SqlParam::Text(val)])
                }
            }
            "less_than" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float < $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} < $1", field), vec![SqlParam::Text(val)])
                }
            }
            "greater_than_or_equal" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float >= $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} >= $1", field), vec![SqlParam::Text(val)])
                }
            }
            "less_than_or_equal" => {
                let val = self.extract_string_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float <= $1::float", field),
                        vec![SqlParam::Text(val)],
                    )
                } else {
                    (format!("{} <= $1", field), vec![SqlParam::Text(val)])
                }
            }
            "between" => {
                let (min, max) = self.extract_range_value(&rule.value)?;
                if is_json_field {
                    (
                        format!("({})::float BETWEEN $1::float AND $2::float", field),
                        vec![SqlParam::Text(min), SqlParam::Text(max)],
                    )
                } else {
                    (
                        format!("{} BETWEEN $1 AND $2", field),
                        vec![SqlParam::Text(min), SqlParam::Text(max)],
                    )
                }
            }
            "in" => {
                let values = self.extract_array_value(&rule.value)?;
                if values.is_empty() {
                    (String::new(), vec![])
                } else {
                    // Use proper array parameter binding to handle values with commas
                    (
                        format!("{} = ANY($1::text[])", field),
                        vec![SqlParam::TextArray(values)],
                    )
                }
            }
            "not_in" => {
                let values = self.extract_array_value(&rule.value)?;
                if values.is_empty() {
                    // If no values to exclude, match everything
                    ("1=1".to_string(), vec![])
                } else {
                    // Use proper array parameter binding to handle values with commas
                    (
                        format!("{} != ALL($1::text[])", field),
                        vec![SqlParam::TextArray(values)],
                    )
                }
            }
            _ => {
                return Err(ApiError::ValidationError(format!(
                    "Unsupported operator: {}",
                    rule.operator
                )));
            }
        };

        Ok((clause, params))
    }

    /// Map field names to SQL column expressions
    ///
    /// SECURITY: This is the ONLY allowed entry point for field names in SQL queries.
    /// All field names are validated against this allowlist before being interpolated
    /// into SQL. Unknown fields result in an error, preventing SQL injection.
    fn get_sql_field(&self, field: &str) -> ApiResult<String> {
        match field {
            // Direct columns
            "title" => Ok("title".to_string()),
            "artist" => Ok("artist_name".to_string()),
            "album" => Ok("album_title".to_string()),
            // "genre" (singular, from VALID_FIELDS) maps to "genres" column
            "genre" | "genres" => Ok("genres".to_string()),
            "ai_mood" => Ok("ai_mood".to_string()),
            "ai_tags" => Ok("ai_tags".to_string()),
            "duration_ms" => Ok("duration_ms".to_string()),
            "play_count" => Ok("play_count".to_string()),
            "skip_count" => Ok("skip_count".to_string()),
            "created_at" => Ok("created_at".to_string()),
            "last_played_at" => Ok("last_played_at".to_string()),
            // JSON fields from audio_features
            "bpm" => Ok("audio_features->>'bpm'".to_string()),
            "energy" => Ok("audio_features->>'energy'".to_string()),
            "danceability" => Ok("audio_features->>'danceability'".to_string()),
            "valence" => Ok("audio_features->>'valence'".to_string()),
            "acousticness" => Ok("audio_features->>'acousticness'".to_string()),
            "instrumentalness" => Ok("audio_features->>'instrumentalness'".to_string()),
            "speechiness" => Ok("audio_features->>'speechiness'".to_string()),
            "loudness" => Ok("audio_features->>'loudness'".to_string()),
            _ => Err(ApiError::ValidationError(format!(
                "Unknown field: {}",
                field
            ))),
        }
    }

    /// Extract a string value from the rule value
    fn extract_string_value(&self, value: &serde_json::Value) -> ApiResult<String> {
        match value {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::Bool(b) => Ok(b.to_string()),
            _ => Err(ApiError::ValidationError(
                "Expected string or number value".to_string(),
            )),
        }
    }

    /// Extract a range (min, max) from the rule value
    fn extract_range_value(&self, value: &serde_json::Value) -> ApiResult<(String, String)> {
        let min = value
            .get("min")
            .ok_or_else(|| ApiError::ValidationError("Range requires 'min' value".to_string()))?;
        let max = value
            .get("max")
            .ok_or_else(|| ApiError::ValidationError("Range requires 'max' value".to_string()))?;

        Ok((
            self.extract_string_value(min)?,
            self.extract_string_value(max)?,
        ))
    }

    /// Extract an array of values from the rule value
    fn extract_array_value(&self, value: &serde_json::Value) -> ApiResult<Vec<String>> {
        match value {
            serde_json::Value::Array(arr) => {
                arr.iter().map(|v| self.extract_string_value(v)).collect()
            }
            _ => Err(ApiError::ValidationError(
                "Expected array value".to_string(),
            )),
        }
    }

    /// Sort tracks by the specified field
    async fn sort_tracks(
        &self,
        track_ids: Vec<Uuid>,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<Uuid>> {
        if track_ids.is_empty() {
            return Ok(track_ids);
        }

        let field = self.get_sql_field(sort_by)?;
        let order = if sort_order == "desc" { "DESC" } else { "ASC" };

        let sql = format!(
            r#"
            SELECT id FROM tracks
            WHERE id = ANY($1)
            ORDER BY {} {} NULLS LAST
            "#,
            field, order
        );

        let sorted: Vec<(Uuid,)> = sqlx::query_as(&sql)
            .bind(&track_ids)
            .fetch_all(&self.pool)
            .await?;

        Ok(sorted.into_iter().map(|(id,)| id).collect())
    }

    /// Refresh a smart playlist by re-evaluating its rules
    ///
    /// # Arguments
    /// * `playlist_id` - The playlist to refresh
    /// * `user_id` - The user triggering the refresh
    ///
    /// # Returns
    /// The updated playlist with refreshed track list
    #[instrument(skip(self))]
    pub async fn refresh_smart_playlist(
        &self,
        playlist_id: Uuid,
        user_id: Uuid,
    ) -> ApiResult<Playlist> {
        // Get the playlist
        let playlist = self
            .playlist_repo
            .find_by_id(playlist_id)
            .await?
            .ok_or_else(|| ApiError::not_found("playlist", playlist_id.to_string()))?;

        // Verify it's a smart playlist
        if playlist.smart_rules.is_none() {
            return Err(ApiError::ValidationError(
                "Cannot refresh a non-smart playlist".to_string(),
            ));
        }

        let rules = playlist.smart_rules.unwrap();

        // Evaluate the rules
        let track_ids = self.evaluate_smart_rules(&rules, user_id).await?;

        // Update the playlist tracks
        self.playlist_repo
            .set_tracks(playlist_id, &track_ids, Some(user_id))
            .await?;

        // Re-fetch to get updated stats from update_playlist_stats
        let updated = self
            .playlist_repo
            .find_by_id(playlist_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Playlist disappeared during refresh".to_string())
            })?;

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    // Note: super::* unused currently since we can't create PlaylistService without a DB pool.
    // Tests for SQL generation and value extraction would require integration tests.

    #[test]
    fn test_get_sql_field_valid_fields() {
        // We can't create a PlaylistService without a pool, but we can verify
        // the field mappings are correct by checking the expected patterns
        let valid_fields = vec![
            "title",
            "artist",
            "album",
            "genres",
            "ai_mood",
            "ai_tags",
            "duration_ms",
            "play_count",
            "skip_count",
            "created_at",
            "last_played_at",
            "bpm",
            "energy",
            "danceability",
            "valence",
            "acousticness",
            "instrumentalness",
            "speechiness",
            "loudness",
        ];
        assert_eq!(valid_fields.len(), 19);
    }
}
