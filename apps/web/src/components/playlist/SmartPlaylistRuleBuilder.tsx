/**
 * Smart Playlist Rule Builder Component
 *
 * A comprehensive component for building smart playlist rules with:
 * - Rule list with add/remove functionality
 * - Match mode toggle (ALL/ANY)
 * - Track limit and sort options
 */

import { memo, useCallback, useRef, useState, useEffect } from 'react'
import { Plus } from 'lucide-react'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/Select'
import { SmartRuleRow } from './SmartRuleRow'
import {
  type SmartRule,
  type SmartRuleField,
  type SmartRuleOperator,
  type SmartRuleValue,
  type SmartMatchMode,
  type SmartPlaylistSort,
  VALIDATION_LIMITS,
  SORT_OPTIONS,
  createDefaultRule,
} from '../../types/playlist'

// ============================================================================
// Types
// ============================================================================

interface SmartPlaylistRuleBuilderProps {
  /** Current list of rules */
  rules: SmartRule[]
  /** Callback when rules change */
  onRulesChange: (rules: SmartRule[]) => void
  /** Current match mode (all/any) */
  matchMode: SmartMatchMode
  /** Callback when match mode changes */
  onMatchModeChange: (mode: SmartMatchMode) => void
  /** Maximum number of tracks to include */
  limit: number
  /** Callback when limit changes */
  onLimitChange: (limit: number) => void
  /** Sort field for ordering tracks */
  sortField: SmartPlaylistSort
  /** Callback when sort field changes */
  onSortFieldChange: (field: SmartPlaylistSort) => void
  /** Sort direction */
  sortDirection: 'asc' | 'desc'
  /** Callback when sort direction changes */
  onSortDirectionChange: (direction: 'asc' | 'desc') => void
  /** Whether the builder is disabled */
  disabled?: boolean
}

// ============================================================================
// Match Mode Toggle
// ============================================================================

interface MatchModeToggleProps {
  value: SmartMatchMode
  onChange: (mode: SmartMatchMode) => void
  disabled: boolean
}

const MatchModeToggle = memo(function MatchModeToggle({
  value,
  onChange,
  disabled,
}: MatchModeToggleProps): JSX.Element {
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm text-text-primary">Match</span>
      <Select
        value={value}
        onValueChange={(v) => onChange(v as SmartMatchMode)}
        disabled={disabled}
      >
        <SelectTrigger className="w-24" aria-label="Match mode">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">ALL</SelectItem>
          <SelectItem value="any">ANY</SelectItem>
        </SelectContent>
      </Select>
      <span className="text-sm text-text-muted">of the following rules</span>
    </div>
  )
})

// ============================================================================
// Options Section
// ============================================================================

interface OptionsSectionProps {
  limit: number
  onLimitChange: (limit: number) => void
  sortField: SmartPlaylistSort
  onSortFieldChange: (field: SmartPlaylistSort) => void
  sortDirection: 'asc' | 'desc'
  onSortDirectionChange: (direction: 'asc' | 'desc') => void
  disabled: boolean
}

const OptionsSection = memo(function OptionsSection({
  limit,
  onLimitChange,
  sortField,
  onSortFieldChange,
  sortDirection,
  onSortDirectionChange,
  disabled,
}: OptionsSectionProps): JSX.Element {
  // Local state for limit input to allow empty/intermediate values while typing
  const [limitText, setLimitText] = useState(String(limit))

  // Sync local state when prop changes (e.g., from parent reset)
  useEffect(() => {
    setLimitText(String(limit))
  }, [limit])

  return (
    <div className="flex flex-wrap items-center gap-4 pt-4 border-t border-white/10">
      {/* Limit */}
      <div className="flex items-center gap-2">
        <label htmlFor="playlist-limit" className="text-sm text-text-primary">
          Max tracks:
        </label>
        <Input
          id="playlist-limit"
          type="number"
          value={limitText}
          onChange={(e) => {
            const next = e.target.value
            setLimitText(next)
            // Commit valid values immediately for responsiveness
            const parsed = parseInt(next, 10)
            if (Number.isFinite(parsed) && parsed >= 1 && parsed <= VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT) {
              onLimitChange(parsed)
            }
          }}
          onBlur={() => {
            // Coerce to valid range on blur and sync local state
            const parsed = parseInt(limitText, 10)
            if (!Number.isFinite(parsed) || parsed < 1) {
              onLimitChange(1)
              setLimitText('1')
            } else if (parsed > VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT) {
              onLimitChange(VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT)
              setLimitText(String(VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT))
            } else {
              onLimitChange(parsed)
              setLimitText(String(parsed))
            }
          }}
          min={1}
          max={VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT}
          disabled={disabled}
          className="w-20"
          aria-describedby="limit-hint"
        />
        <span id="limit-hint" className="text-xs text-text-muted">
          (1-{VALIDATION_LIMITS.MAX_PLAYLIST_LIMIT.toLocaleString()})
        </span>
      </div>

      {/* Sort */}
      <div className="flex items-center gap-2">
        <label className="text-sm text-text-primary">Sort by:</label>
        <Select
          value={sortField}
          onValueChange={(v) => onSortFieldChange(v as SmartPlaylistSort)}
          disabled={disabled}
        >
          <SelectTrigger className="w-36" aria-label="Sort field">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SORT_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select
          value={sortDirection}
          onValueChange={(v) => onSortDirectionChange(v as 'asc' | 'desc')}
          disabled={disabled}
        >
          <SelectTrigger className="w-28" aria-label="Sort direction">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="desc">Descending</SelectItem>
            <SelectItem value="asc">Ascending</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
})

// ============================================================================
// Main Component
// ============================================================================

export const SmartPlaylistRuleBuilder = memo(function SmartPlaylistRuleBuilder({
  rules,
  onRulesChange,
  matchMode,
  onMatchModeChange,
  limit,
  onLimitChange,
  sortField,
  onSortFieldChange,
  sortDirection,
  onSortDirectionChange,
  disabled = false,
}: SmartPlaylistRuleBuilderProps): JSX.Element {
  // Use ref to avoid callback recreation when rules change
  const rulesRef = useRef(rules)
  rulesRef.current = rules

  // Check if we can add more rules
  const canAddRule = rules.length < VALIDATION_LIMITS.MAX_RULES

  // Check if we can delete rules (must have at least one)
  const canDeleteRule = rules.length > 1

  // Add a new rule
  const handleAddRule = useCallback(() => {
    if (rulesRef.current.length >= VALIDATION_LIMITS.MAX_RULES) return
    const newRule = createDefaultRule()
    onRulesChange([...rulesRef.current, newRule])
  }, [onRulesChange])

  // Delete a rule
  const handleDeleteRule = useCallback(
    (id: string) => {
      if (rulesRef.current.length <= 1) return
      onRulesChange(rulesRef.current.filter((r) => r.id !== id))
    },
    [onRulesChange]
  )

  // Update rule field
  const handleFieldChange = useCallback(
    (id: string, field: SmartRuleField) => {
      onRulesChange(
        rulesRef.current.map((r) => (r.id === id ? { ...r, field } : r))
      )
    },
    [onRulesChange]
  )

  // Update rule operator
  const handleOperatorChange = useCallback(
    (id: string, operator: SmartRuleOperator) => {
      onRulesChange(
        rulesRef.current.map((r) => (r.id === id ? { ...r, operator } : r))
      )
    },
    [onRulesChange]
  )

  // Update rule value
  const handleValueChange = useCallback(
    (id: string, value: SmartRuleValue) => {
      onRulesChange(
        rulesRef.current.map((r) => (r.id === id ? { ...r, value } : r))
      )
    },
    [onRulesChange]
  )

  // Simple string interpolation doesn't need memoization
  const ruleCountDisplay = `${rules.length}/${VALIDATION_LIMITS.MAX_RULES} rules`

  return (
    <div className="flex flex-col gap-4">
      {/* Header with match mode */}
      <div className="flex items-center justify-between">
        <MatchModeToggle
          value={matchMode}
          onChange={onMatchModeChange}
          disabled={disabled}
        />
        <span
          className="text-xs text-text-muted"
          aria-live="polite"
          aria-atomic="true"
        >
          {ruleCountDisplay}
        </span>
      </div>

      {/* Rules list */}
      <div className="flex flex-col" role="list" aria-label="Playlist rules">
        {rules.map((rule, index) => (
          <SmartRuleRow
            key={rule.id}
            rule={rule}
            index={index}
            onFieldChange={handleFieldChange}
            onOperatorChange={handleOperatorChange}
            onValueChange={handleValueChange}
            onDelete={handleDeleteRule}
            canDelete={canDeleteRule}
            disabled={disabled}
          />
        ))}
      </div>

      {/* Add rule button */}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={handleAddRule}
        disabled={disabled || !canAddRule}
        className="self-start"
      >
        <Plus size={16} className="mr-1" aria-hidden="true" />
        Add Rule
      </Button>

      {/* Options section */}
      <OptionsSection
        limit={limit}
        onLimitChange={onLimitChange}
        sortField={sortField}
        onSortFieldChange={onSortFieldChange}
        sortDirection={sortDirection}
        onSortDirectionChange={onSortDirectionChange}
        disabled={disabled}
      />
    </div>
  )
})

// Export component types for testing
export type { SmartPlaylistRuleBuilderProps }
