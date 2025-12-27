/**
 * API response and error types for the Resonance API
 */

// ============================================================================
// Error Types
// ============================================================================

/**
 * Standard API error response
 */
export interface ApiError {
  /** Machine-readable error code */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Additional error context */
  details?: Record<string, unknown>;
  /** HTTP status code */
  statusCode?: number;
}

/**
 * Validation error with field-specific messages
 */
export interface ValidationError extends ApiError {
  code: 'VALIDATION_ERROR';
  /** Field-specific validation errors */
  fieldErrors: Record<string, string[]>;
}

/**
 * Authentication error
 */
export interface AuthError extends ApiError {
  code: 'UNAUTHORIZED' | 'TOKEN_EXPIRED' | 'INVALID_CREDENTIALS' | 'SESSION_EXPIRED';
}

/**
 * Rate limiting error
 */
export interface RateLimitError extends ApiError {
  code: 'RATE_LIMITED';
  /** Seconds until rate limit resets */
  retryAfter: number;
}

// ============================================================================
// Response Types
// ============================================================================

/**
 * Generic paginated response wrapper
 */
export interface PaginatedResponse<T> {
  /** Array of items for the current page */
  items: T[];
  /** Total number of items across all pages */
  total: number;
  /** Current offset (starting index) */
  offset: number;
  /** Maximum items per page */
  limit: number;
  /** Whether more items are available */
  hasMore: boolean;
}

/**
 * API response wrapper with success/error discrimination
 */
export type ApiResponse<T> =
  | { success: true; data: T }
  | { success: false; error: ApiError };

/**
 * Health check response
 */
export interface HealthCheckResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  version: string;
  uptime: number;
  services: {
    database: ServiceHealth;
    redis: ServiceHealth;
    meilisearch: ServiceHealth;
    ollama: ServiceHealth;
  };
}

/**
 * Individual service health status
 */
export interface ServiceHealth {
  status: 'up' | 'down' | 'degraded';
  latency?: number;
  message?: string;
}

// ============================================================================
// Request Types
// ============================================================================

/**
 * Standard pagination parameters
 */
export interface PaginationParams {
  offset?: number;
  limit?: number;
}

/**
 * Sort parameters
 */
export interface SortParams<T extends string = string> {
  sortBy?: T;
  sortOrder?: 'asc' | 'desc';
}

/**
 * Search query parameters
 */
export interface SearchParams extends PaginationParams {
  query: string;
  filters?: Record<string, string | string[]>;
}

// ============================================================================
// Streaming Types
// ============================================================================

/**
 * Audio streaming request parameters
 */
export interface StreamRequest {
  /** Track ID to stream */
  trackId: string;
  /** Requested audio quality */
  quality?: 'low' | 'medium' | 'high' | 'lossless';
  /** Starting position in seconds */
  startPosition?: number;
}

/**
 * Audio stream metadata returned with stream response
 */
export interface StreamMetadata {
  /** Content-Type header value */
  contentType: string;
  /** Total content length in bytes */
  contentLength: number;
  /** Duration in seconds */
  duration: number;
  /** Bitrate in kbps */
  bitrate: number;
  /** Sample rate in Hz */
  sampleRate: number;
  /** Whether range requests are supported */
  acceptRanges: boolean;
}
