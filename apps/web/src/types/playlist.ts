/**
 * Smart Playlist Types
 *
 * Type definitions for smart playlist creation and rule configuration.
 * Maps to backend GraphQL schema in apps/api/src/graphql/mutation/playlist.rs
 */

// ============================================================================
// Smart Rule Field Types
// ============================================================================

/** All valid fields that can be used in smart playlist rules */
export type SmartRuleField =
  // Metadata fields
  | 'title'
  | 'artist'
  | 'album'
  | 'genre'
  | 'genres'
  // Audio feature fields
  | 'duration_ms'
  | 'bpm'
  | 'energy'
  | 'danceability'
  | 'valence'
  | 'acousticness'
  | 'instrumentalness'
  | 'speechiness'
  | 'loudness'
  // AI fields
  | 'ai_mood'
  | 'ai_tags'
  // Stats fields
  | 'play_count'
  | 'skip_count'
  | 'created_at'
  | 'last_played_at'
  // Special fields
  | 'similar_to'

// ============================================================================
// Operator Types
// ============================================================================

/** Operators for string-based fields */
export type StringOperator =
  | 'equals'
  | 'not_equals'
  | 'contains'
  | 'not_contains'
  | 'starts_with'
  | 'ends_with'
  | 'in'
  | 'not_in'
  | 'is_empty'

/** Operators for numeric fields */
export type NumericOperator =
  | 'equals'
  | 'not_equals'
  | 'greater_than'
  | 'less_than'
  | 'greater_than_or_equal'
  | 'less_than_or_equal'
  | 'between'

/** Operators for array fields (genres, moods, tags) */
export type ArrayOperator =
  | 'contains'
  | 'not_contains'
  | 'in'
  | 'not_in'
  | 'is_empty'

/** Operators for similarity-based matching */
export type SimilarityOperator =
  | 'combined'
  | 'semantic'
  | 'acoustic'
  | 'categorical'

/** Union of all operator types */
export type SmartRuleOperator =
  | StringOperator
  | NumericOperator
  | ArrayOperator
  | SimilarityOperator

// ============================================================================
// Value Types
// ============================================================================

/** Range value for 'between' operator on numeric fields */
export interface RangeValue {
  min: number
  max: number
}

/** Date range value for 'between' operator on date fields */
export interface DateRangeValue {
  min: string // ISO date string (YYYY-MM-DD)
  max: string // ISO date string (YYYY-MM-DD)
}

/** Value for similarity-based rules */
export interface SimilarityValue {
  track_ids: string[]
  min_score?: number
}

/** All possible rule value types */
export type SmartRuleValue =
  | string
  | number
  | string[]
  | RangeValue
  | DateRangeValue
  | SimilarityValue
  | null

// ============================================================================
// Type Guards for SmartRuleValue
// ============================================================================

/** Type guard for string values */
export function isStringValue(value: SmartRuleValue): value is string {
  return typeof value === 'string'
}

/** Type guard for number values */
export function isNumberValue(value: SmartRuleValue): value is number {
  return typeof value === 'number'
}

/** Type guard for string array values */
export function isStringArray(value: SmartRuleValue): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === 'string')
}

/** Type guard for numeric range values (between operator) */
export function isRangeValue(value: SmartRuleValue): value is RangeValue {
  return (
    value !== null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    'min' in value &&
    'max' in value &&
    typeof value.min === 'number' &&
    typeof value.max === 'number'
  )
}

/** Type guard for date range values (between operator on date fields) */
export function isDateRangeValue(value: SmartRuleValue): value is DateRangeValue {
  return (
    value !== null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    'min' in value &&
    'max' in value &&
    typeof value.min === 'string' &&
    typeof value.max === 'string'
  )
}

/** Type guard for similarity values */
export function isSimilarityValue(value: SmartRuleValue): value is SimilarityValue {
  return (
    value !== null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    'track_ids' in value &&
    Array.isArray(value.track_ids)
  )
}

// ============================================================================
// Rule Configuration
// ============================================================================

/** A single smart playlist rule */
export interface SmartRule {
  id: string
  field: SmartRuleField
  operator: SmartRuleOperator
  value: SmartRuleValue
}

/** Match mode for combining rules */
export type MatchMode = 'all' | 'any'

/** Alias for SmartMatchMode used in components */
export type SmartMatchMode = MatchMode

/** Sort order for playlist tracks */
export type SortOrder = 'asc' | 'desc'

/** Fields available for sorting smart playlist results */
export type SmartPlaylistSort =
  | 'title'
  | 'artist'
  | 'album'
  | 'duration_ms'
  | 'bpm'
  | 'energy'
  | 'danceability'
  | 'valence'
  | 'play_count'
  | 'created_at'
  | 'random'

/** Sort options for the UI dropdown */
export const SORT_OPTIONS: Array<{ value: SmartPlaylistSort; label: string }> = [
  { value: 'random', label: 'Random' },
  { value: 'title', label: 'Title' },
  { value: 'artist', label: 'Artist' },
  { value: 'album', label: 'Album' },
  { value: 'play_count', label: 'Play Count' },
  { value: 'created_at', label: 'Date Added' },
  { value: 'duration_ms', label: 'Duration' },
  { value: 'bpm', label: 'BPM' },
  { value: 'energy', label: 'Energy' },
  { value: 'danceability', label: 'Danceability' },
  { value: 'valence', label: 'Positivity' },
]

// ============================================================================
// Field Configuration for UI
// ============================================================================

/** Category groupings for the field selector dropdown */
export type FieldCategory = 'metadata' | 'audio' | 'ai' | 'stats' | 'special'

/** Value type determines which input component to render */
export type ValueType = 'string' | 'number' | 'array' | 'date' | 'similar_to'

/** Configuration for a single field in the UI */
export interface FieldConfig {
  field: SmartRuleField
  label: string
  category: FieldCategory
  valueType: ValueType
  operators: SmartRuleOperator[]
  unit?: string
  min?: number
  max?: number
  step?: number
  description?: string
}

/** Complete field configuration for the smart playlist UI */
export const SMART_RULE_FIELDS: FieldConfig[] = [
  // Metadata fields
  {
    field: 'title',
    label: 'Title',
    category: 'metadata',
    valueType: 'string',
    operators: ['equals', 'not_equals', 'contains', 'not_contains', 'starts_with', 'ends_with'],
    description: 'Track title',
  },
  {
    field: 'artist',
    label: 'Artist',
    category: 'metadata',
    valueType: 'string',
    operators: ['equals', 'not_equals', 'contains', 'not_contains', 'starts_with', 'ends_with'],
    description: 'Artist name',
  },
  {
    field: 'album',
    label: 'Album',
    category: 'metadata',
    valueType: 'string',
    operators: ['equals', 'not_equals', 'contains', 'not_contains', 'starts_with', 'ends_with'],
    description: 'Album title',
  },
  {
    field: 'genre',
    label: 'Genre',
    category: 'metadata',
    valueType: 'string',
    operators: ['equals', 'not_equals', 'contains', 'in', 'not_in', 'is_empty'],
    description: 'Music genre',
  },
  {
    field: 'genres',
    label: 'Genres',
    category: 'metadata',
    valueType: 'array',
    operators: ['contains', 'not_contains', 'in', 'not_in', 'is_empty'],
    description: 'All genre tags assigned to the track',
  },

  // Audio feature fields
  {
    field: 'bpm',
    label: 'BPM',
    category: 'audio',
    valueType: 'number',
    operators: ['equals', 'greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 40,
    max: 220,
    step: 1,
    unit: 'bpm',
    description: 'Beats per minute (tempo)',
  },
  {
    field: 'energy',
    label: 'Energy',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'Intensity and activity level',
  },
  {
    field: 'danceability',
    label: 'Danceability',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'How suitable for dancing',
  },
  {
    field: 'valence',
    label: 'Positivity',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'Musical positiveness (happy vs sad)',
  },
  {
    field: 'acousticness',
    label: 'Acousticness',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'Acoustic vs electronic',
  },
  {
    field: 'instrumentalness',
    label: 'Instrumentalness',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'Likelihood of no vocals',
  },
  {
    field: 'speechiness',
    label: 'Speechiness',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 100,
    step: 1,
    unit: '%',
    description: 'Presence of spoken words',
  },
  {
    field: 'loudness',
    label: 'Loudness',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: -60,
    max: 0,
    step: 1,
    unit: 'dB',
    description: 'Overall loudness in decibels',
  },
  {
    field: 'duration_ms',
    label: 'Duration',
    category: 'audio',
    valueType: 'number',
    operators: ['greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    max: 3600000,
    step: 1000,
    unit: 'ms',
    description: 'Track length in milliseconds',
  },

  // AI fields
  {
    field: 'ai_mood',
    label: 'Mood',
    category: 'ai',
    valueType: 'array',
    operators: ['contains', 'not_contains', 'in', 'not_in', 'is_empty'],
    description: 'AI-detected mood tags',
  },
  {
    field: 'ai_tags',
    label: 'AI Tags',
    category: 'ai',
    valueType: 'array',
    operators: ['contains', 'not_contains', 'in', 'not_in', 'is_empty'],
    description: 'AI-generated semantic tags',
  },

  // Stats fields
  {
    field: 'play_count',
    label: 'Play Count',
    category: 'stats',
    valueType: 'number',
    operators: ['equals', 'greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    step: 1,
    description: 'Number of times played',
  },
  {
    field: 'skip_count',
    label: 'Skip Count',
    category: 'stats',
    valueType: 'number',
    operators: ['equals', 'greater_than', 'less_than', 'greater_than_or_equal', 'less_than_or_equal', 'between'],
    min: 0,
    step: 1,
    description: 'Number of times skipped',
  },
  {
    field: 'created_at',
    label: 'Date Added',
    category: 'stats',
    valueType: 'date',
    operators: ['greater_than', 'less_than', 'between'],
    description: 'When the track was added to library',
  },
  {
    field: 'last_played_at',
    label: 'Last Played',
    category: 'stats',
    valueType: 'date',
    operators: ['greater_than', 'less_than', 'between', 'is_empty'],
    description: 'When the track was last played (empty = never played)',
  },

  // Special fields
  {
    field: 'similar_to',
    label: 'Similar to Tracks',
    category: 'special',
    valueType: 'similar_to',
    operators: ['combined', 'semantic', 'acoustic', 'categorical'],
    description: 'Find tracks similar to selected seeds',
  },
]

/** Get field configuration by field name */
export function getFieldConfig(field: SmartRuleField): FieldConfig | undefined {
  return SMART_RULE_FIELDS.find((f) => f.field === field)
}

/**
 * Generate a unique ID for a new rule
 */
export function generateRuleId(): string {
  return `rule-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

/**
 * Create a new default rule with sensible defaults
 */
export function createDefaultRule(): SmartRule {
  const defaultField: SmartRuleField = 'genre'
  const fieldConfig = getFieldConfig(defaultField)
  const defaultOperator = fieldConfig?.operators[0] ?? 'contains'

  return {
    id: generateRuleId(),
    field: defaultField,
    operator: defaultOperator,
    value: '',
  }
}

/** Get fields grouped by category */
export function getFieldsByCategory(): Record<FieldCategory, FieldConfig[]> {
  return SMART_RULE_FIELDS.reduce(
    (acc, field) => {
      acc[field.category].push(field)
      return acc
    },
    {
      metadata: [],
      audio: [],
      ai: [],
      stats: [],
      special: [],
    } as Record<FieldCategory, FieldConfig[]>
  )
}

/** Operator labels for display */
export const OPERATOR_LABELS: Record<SmartRuleOperator, string> = {
  // String operators
  equals: 'equals',
  not_equals: 'does not equal',
  contains: 'contains',
  not_contains: 'does not contain',
  starts_with: 'starts with',
  ends_with: 'ends with',
  in: 'is one of',
  not_in: 'is not one of',
  is_empty: 'is empty',
  // Numeric operators
  greater_than: 'is greater than',
  less_than: 'is less than',
  greater_than_or_equal: 'is at least',
  less_than_or_equal: 'is at most',
  between: 'is between',
  // Similarity operators
  combined: 'combined similarity',
  semantic: 'semantic similarity',
  acoustic: 'acoustic similarity',
  categorical: 'categorical similarity',
}

/** Category labels for display */
export const CATEGORY_LABELS: Record<FieldCategory, string> = {
  metadata: 'Metadata',
  audio: 'Audio Features',
  ai: 'AI Analysis',
  stats: 'Statistics',
  special: 'Special',
}

// ============================================================================
// Validation Constants
// ============================================================================

/** Validation limits matching backend constraints */
export const VALIDATION_LIMITS = {
  MAX_NAME_LENGTH: 255,
  MAX_DESCRIPTION_LENGTH: 2000,
  MAX_RULES: 50,
  MAX_TRACK_LIMIT: 10000,
  MAX_PLAYLIST_LIMIT: 10000,
  MAX_SEED_TRACKS: 10,
  DEFAULT_PLAYLIST_LIMIT: 100,
} as const

// ============================================================================
// GraphQL Input Types
// ============================================================================

/** All playlist types supported by the backend */
export type PlaylistTypeValue = 'Manual' | 'Smart' | 'Discover' | 'Radio'

/** Input for a single smart playlist rule (GraphQL) */
export interface SmartPlaylistRuleInput {
  field: SmartRuleField
  operator: SmartRuleOperator
  value: SmartRuleValue
}

/** Input for smart playlist rules configuration (GraphQL) */
export interface SmartPlaylistRulesInput {
  matchMode: MatchMode
  rules: SmartPlaylistRuleInput[]
  limit?: number
  sortBy?: SmartRuleField
  sortOrder?: SortOrder
}

/** Input for creating a playlist (GraphQL) */
export interface CreatePlaylistInput {
  name: string
  description?: string
  isPublic: boolean
  playlistType: PlaylistTypeValue
  smartRules?: SmartPlaylistRulesInput
}

/** Response from createPlaylist mutation */
export interface CreatePlaylistResponse {
  createPlaylist: {
    id: string
    name: string
    description?: string
    isPublic: boolean
    playlistType: string
    trackCount: number
    createdAt: string
  }
}

/** Response from refreshSmartPlaylist mutation */
export interface RefreshSmartPlaylistResponse {
  refreshSmartPlaylist: {
    id: string
    trackCount: number
    totalDurationMs: number
    formattedDuration: string
    updatedAt: string
  }
}

// ============================================================================
// Form State Types
// ============================================================================

/** Form state for the smart playlist creator modal */
export interface SmartPlaylistFormState {
  name: string
  description: string
  isPublic: boolean
  matchMode: MatchMode
  rules: SmartRule[]
  limit: number | null
  sortBy: SmartRuleField | null
  sortOrder: SortOrder | null
}

/** Initial empty form state */
export const INITIAL_FORM_STATE: SmartPlaylistFormState = {
  name: '',
  description: '',
  isPublic: false,
  matchMode: 'all',
  rules: [],
  limit: null,
  sortBy: null,
  sortOrder: null,
}

// ============================================================================
// Validation
// ============================================================================

/** Validation error for form fields */
export interface ValidationError {
  field: string
  message: string
}

/** Validate the smart playlist form */
export function validateSmartPlaylistForm(form: SmartPlaylistFormState): ValidationError | null {
  const { MAX_NAME_LENGTH, MAX_DESCRIPTION_LENGTH, MAX_RULES, MAX_TRACK_LIMIT, MAX_SEED_TRACKS } = VALIDATION_LIMITS

  // Name validation
  if (!form.name.trim()) {
    return { field: 'name', message: 'Playlist name is required' }
  }
  if (form.name.length > MAX_NAME_LENGTH) {
    return { field: 'name', message: `Name cannot exceed ${MAX_NAME_LENGTH} characters` }
  }

  // Description validation
  if (form.description && form.description.length > MAX_DESCRIPTION_LENGTH) {
    return { field: 'description', message: `Description cannot exceed ${MAX_DESCRIPTION_LENGTH} characters` }
  }

  // Rules validation
  if (form.rules.length === 0) {
    return { field: 'rules', message: 'At least one rule is required' }
  }
  if (form.rules.length > MAX_RULES) {
    return { field: 'rules', message: `Cannot exceed ${MAX_RULES} rules` }
  }

  // Validate each rule
  for (const [i, rule] of form.rules.entries()) {
    if (!rule.field) {
      return { field: `rules.${i}.field`, message: `Rule ${i + 1}: Field is required` }
    }
    if (!rule.operator) {
      return { field: `rules.${i}.operator`, message: `Rule ${i + 1}: Operator is required` }
    }

    // Value validation based on field type
    if (rule.field === 'similar_to') {
      const val = rule.value as SimilarityValue | null
      if (!val?.track_ids || val.track_ids.length === 0) {
        return { field: `rules.${i}.value`, message: `Rule ${i + 1}: At least one seed track is required` }
      }
      if (val.track_ids.length > MAX_SEED_TRACKS) {
        return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Cannot have more than ${MAX_SEED_TRACKS} seed tracks` }
      }
    } else if (
      rule.operator !== 'is_empty' &&
      rule.operator !== 'between' &&
      (rule.value === null ||
        (typeof rule.value === 'string' && rule.value.trim() === '') ||
        (Array.isArray(rule.value) && rule.value.length === 0))
    ) {
      return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Value is required` }
    }

    // Range validation for 'between' operator
    if (rule.operator === 'between') {
      if (!isRangeValue(rule.value) && !isDateRangeValue(rule.value)) {
        return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Range is required` }
      }

      if (isRangeValue(rule.value)) {
        if (!Number.isFinite(rule.value.min) || !Number.isFinite(rule.value.max)) {
          return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Range values must be numbers` }
        }
        if (rule.value.min > rule.value.max) {
          return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Minimum cannot exceed maximum` }
        }
      }

      if (isDateRangeValue(rule.value)) {
        if (!rule.value.min || !rule.value.max) {
          return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Start and end dates are required` }
        }
        const minTime = Date.parse(rule.value.min)
        const maxTime = Date.parse(rule.value.max)
        if (!Number.isFinite(minTime) || !Number.isFinite(maxTime)) {
          return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Dates must be valid` }
        }
        if (minTime > maxTime) {
          return { field: `rules.${i}.value`, message: `Rule ${i + 1}: Start date must be before end date` }
        }
      }
    }
  }

  // Limit validation
  if (form.limit !== null) {
    if (form.limit <= 0) {
      return { field: 'limit', message: 'Limit must be a positive number' }
    }
    if (form.limit > MAX_TRACK_LIMIT) {
      return { field: 'limit', message: `Limit cannot exceed ${MAX_TRACK_LIMIT.toLocaleString()} tracks` }
    }
  }

  return null
}

/** Convert form state to GraphQL input */
export function formStateToInput(form: SmartPlaylistFormState): CreatePlaylistInput {
  return {
    name: form.name.trim(),
    description: form.description.trim() || undefined,
    isPublic: form.isPublic,
    playlistType: 'Smart',
    smartRules: {
      matchMode: form.matchMode,
      rules: form.rules.map((rule) => ({
        field: rule.field,
        operator: rule.operator,
        value: rule.value,
      })),
      limit: form.limit ?? undefined,
      sortBy: form.sortBy ?? undefined,
      sortOrder: form.sortOrder ?? undefined,
    },
  }
}

// ============================================================================
// Mutation Input/Response Types
// ============================================================================

/** Input for updating an existing playlist */
export interface UpdatePlaylistInput {
  name?: string
  description?: string
  isPublic?: boolean
  smartRules?: SmartPlaylistRulesInput
}

/** Response from updatePlaylist mutation */
export interface UpdatePlaylistResponse {
  updatePlaylist: {
    id: string
    name: string
    description: string | null
    isPublic: boolean
    playlistType: string
    trackCount: number
    totalDurationMs: number
    formattedDuration: string
    updatedAt: string
  }
}

/** Response from deletePlaylist mutation */
export interface DeletePlaylistResponse {
  deletePlaylist: boolean
}

/** Input for adding tracks to a playlist */
export interface AddTracksInput {
  trackIds: string[]
  position?: number
}

/** Response from addTracksToPlaylist mutation */
export interface AddTracksResponse {
  addTracksToPlaylist: {
    id: string
    trackCount: number
    totalDurationMs: number
    formattedDuration: string
    updatedAt: string
  }
}

/** Input for removing tracks from a playlist */
export interface RemoveTracksInput {
  trackIds: string[]
}

/** Response from removeTracksFromPlaylist mutation */
export interface RemoveTracksResponse {
  removeTracksFromPlaylist: {
    id: string
    trackCount: number
    totalDurationMs: number
    formattedDuration: string
    updatedAt: string
  }
}

/** Variables for updatePlaylist mutation hook */
export interface UpdatePlaylistVariables {
  id: string
  input: UpdatePlaylistInput
}

/** Variables for addTracksToPlaylist mutation hook */
export interface AddTracksToPlaylistVariables {
  playlistId: string
  input: AddTracksInput
}

/** Variables for removeTracksFromPlaylist mutation hook */
export interface RemoveTracksFromPlaylistVariables {
  playlistId: string
  input: RemoveTracksInput
}
