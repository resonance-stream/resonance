/**
 * Smart Rule Row Component
 *
 * Renders a single rule in the smart playlist builder with:
 * - Field selector (grouped by category)
 * - Operator selector (filtered by field type)
 * - Value input (dynamic based on field type)
 * - Delete button
 */

import { memo, useMemo, useCallback } from 'react'
import { Trash2 } from 'lucide-react'
import { Button } from '../ui/Button'
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from '../ui/Select'
import { SmartRuleValueInput } from './SmartRuleValueInput'
import {
  type SmartRule,
  type SmartRuleField,
  type SmartRuleOperator,
  type SmartRuleValue,
  type FieldConfig,
  getFieldsByCategory,
  getFieldConfig,
  OPERATOR_LABELS,
  CATEGORY_LABELS,
} from '../../types/playlist'

// ============================================================================
// Types
// ============================================================================

interface SmartRuleRowProps {
  /** The rule data */
  rule: SmartRule
  /** Index of the rule (for display) */
  index: number
  /** Callback when field changes */
  onFieldChange: (id: string, field: SmartRuleField) => void
  /** Callback when operator changes */
  onOperatorChange: (id: string, operator: SmartRuleOperator) => void
  /** Callback when value changes */
  onValueChange: (id: string, value: SmartRuleValue) => void
  /** Callback to delete the rule */
  onDelete: (id: string) => void
  /** Whether the rule can be deleted (at least one rule required) */
  canDelete: boolean
  /** Whether the inputs are disabled */
  disabled?: boolean
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get the default value for a field type when it changes
 */
function getDefaultValueForField(config: FieldConfig): SmartRuleValue {
  switch (config.valueType) {
    case 'string':
      return ''
    case 'number':
      return config.min ?? 0
    case 'array':
      return []
    case 'date':
      return ''
    case 'similar_to':
      return { track_ids: [] }
    default:
      return null
  }
}

/**
 * Get the default operator for a field
 */
function getDefaultOperator(config: FieldConfig): SmartRuleOperator {
  // All field configs have at least one operator, but TypeScript doesn't know that
  const firstOperator = config.operators[0]
  if (!firstOperator) {
    throw new Error(`Field ${config.field} has no operators configured`)
  }
  return firstOperator
}

// ============================================================================
// Field Selector Component
// ============================================================================

interface FieldSelectorProps {
  value: SmartRuleField
  onChange: (field: SmartRuleField) => void
  disabled: boolean
}

const FieldSelector = memo(function FieldSelector({
  value,
  onChange,
  disabled,
}: FieldSelectorProps): JSX.Element {
  const fieldsByCategory = useMemo(() => getFieldsByCategory(), [])

  return (
    <Select
      value={value}
      onValueChange={(v) => onChange(v as SmartRuleField)}
      disabled={disabled}
    >
      <SelectTrigger className="w-40" aria-label="Select field">
        <SelectValue placeholder="Select field..." />
      </SelectTrigger>
      <SelectContent>
        {Object.entries(fieldsByCategory).map(([category, fields]) => (
          <SelectGroup key={category}>
            <SelectLabel>{CATEGORY_LABELS[category as keyof typeof CATEGORY_LABELS]}</SelectLabel>
            {fields.map((field) => (
              <SelectItem key={field.field} value={field.field}>
                {field.label}
              </SelectItem>
            ))}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  )
})

// ============================================================================
// Operator Selector Component
// ============================================================================

interface OperatorSelectorProps {
  value: SmartRuleOperator
  operators: SmartRuleOperator[]
  onChange: (operator: SmartRuleOperator) => void
  disabled: boolean
}

const OperatorSelector = memo(function OperatorSelector({
  value,
  operators,
  onChange,
  disabled,
}: OperatorSelectorProps): JSX.Element {
  return (
    <Select
      value={value}
      onValueChange={(v) => onChange(v as SmartRuleOperator)}
      disabled={disabled}
    >
      <SelectTrigger className="w-36" aria-label="Select operator">
        <SelectValue placeholder="Select operator..." />
      </SelectTrigger>
      <SelectContent>
        {operators.map((op) => (
          <SelectItem key={op} value={op}>
            {OPERATOR_LABELS[op]}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
})

// ============================================================================
// Main Component
// ============================================================================

export const SmartRuleRow = memo(function SmartRuleRow({
  rule,
  index,
  onFieldChange,
  onOperatorChange,
  onValueChange,
  onDelete,
  canDelete,
  disabled = false,
}: SmartRuleRowProps): JSX.Element {
  // Get field configuration
  const fieldConfig = useMemo(() => getFieldConfig(rule.field), [rule.field])

  // Get valid operators for the current field
  const validOperators = useMemo(
    () => fieldConfig?.operators ?? [],
    [fieldConfig]
  )

  // Handle field change - reset operator and value to defaults
  const handleFieldChange = useCallback(
    (field: SmartRuleField) => {
      const newConfig = getFieldConfig(field)
      if (newConfig) {
        onFieldChange(rule.id, field)
        onOperatorChange(rule.id, getDefaultOperator(newConfig))
        onValueChange(rule.id, getDefaultValueForField(newConfig))
      }
    },
    [rule.id, onFieldChange, onOperatorChange, onValueChange]
  )

  // Handle operator change
  const handleOperatorChange = useCallback(
    (operator: SmartRuleOperator) => {
      onOperatorChange(rule.id, operator)
      // Reset value if changing to/from is_empty (which doesn't need a value)
      if (operator === 'is_empty' || rule.operator === 'is_empty') {
        onValueChange(rule.id, fieldConfig ? getDefaultValueForField(fieldConfig) : null)
      }
    },
    [rule.id, rule.operator, fieldConfig, onOperatorChange, onValueChange]
  )

  // Handle value change
  const handleValueChange = useCallback(
    (value: SmartRuleValue) => {
      onValueChange(rule.id, value)
    },
    [rule.id, onValueChange]
  )

  // Handle delete
  const handleDelete = useCallback(() => {
    onDelete(rule.id)
  }, [rule.id, onDelete])

  return (
    <div
      className="flex items-start gap-2 py-2"
      role="listitem"
      aria-label={`Rule ${index + 1}`}
    >
      {/* Rule number indicator */}
      <span className="flex-shrink-0 w-6 h-9 flex items-center justify-center text-sm text-text-muted">
        {index + 1}.
      </span>

      {/* Field selector */}
      <FieldSelector
        value={rule.field}
        onChange={handleFieldChange}
        disabled={disabled}
      />

      {/* Operator selector */}
      <OperatorSelector
        value={rule.operator}
        operators={validOperators}
        onChange={handleOperatorChange}
        disabled={disabled}
      />

      {/* Value input */}
      {fieldConfig && (
        <div className="flex-1 min-w-0">
          <SmartRuleValueInput
            fieldConfig={fieldConfig}
            operator={rule.operator}
            value={rule.value}
            onChange={handleValueChange}
            disabled={disabled}
          />
        </div>
      )}

      {/* Delete button */}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={handleDelete}
        disabled={disabled || !canDelete}
        className="flex-shrink-0 text-text-muted hover:text-error-text"
        aria-label={`Delete rule ${index + 1}`}
      >
        <Trash2 size={16} />
      </Button>
    </div>
  )
})

// Export component types for testing
export type { SmartRuleRowProps }
