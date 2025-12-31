/**
 * Smart Rule Value Input Component
 *
 * Renders appropriate input controls based on field type and operator.
 * Handles string, number, array, date, and similar_to value types.
 */

import { memo, useState, useCallback, useMemo, useRef, type KeyboardEvent, type JSX } from 'react'
import { X } from 'lucide-react'
import { Input } from '../ui/Input'
import { Slider } from '../ui/Slider'
import { Button } from '../ui/Button'
import { TrackSearchInput } from './TrackSearchInput'
import {
  isStringValue,
  isNumberValue,
  isStringArray,
  isRangeValue,
  isDateRangeValue,
  isSimilarityValue,
  type SmartRuleValue,
  type SmartRuleOperator,
  type SimilarityValue,
  type FieldConfig,
} from '../../types/playlist'

// ============================================================================
// Main Component
// ============================================================================

interface SmartRuleValueInputProps {
  /** Field configuration for the current rule */
  fieldConfig: FieldConfig
  /** Current operator */
  operator: SmartRuleOperator
  /** Current value */
  value: SmartRuleValue
  /** Callback when value changes */
  onChange: (value: SmartRuleValue) => void
  /** Whether the input is disabled */
  disabled?: boolean
}

/**
 * Renders the appropriate input component(s) based on field type and operator
 */
export const SmartRuleValueInput = memo(function SmartRuleValueInput({
  fieldConfig,
  operator,
  value,
  onChange,
  disabled = false,
}: SmartRuleValueInputProps): JSX.Element | null {
  // No value input needed for is_empty operator
  if (operator === 'is_empty') {
    return null
  }

  // Render based on value type
  switch (fieldConfig.valueType) {
    case 'string':
      return (
        <StringValueInput
          value={value}
          onChange={onChange}
          operator={operator}
          disabled={disabled}
        />
      )

    case 'number':
      return (
        <NumericValueInput
          value={value}
          onChange={onChange}
          operator={operator}
          fieldConfig={fieldConfig}
          disabled={disabled}
        />
      )

    case 'array':
      return (
        <ArrayValueInput
          value={value}
          onChange={onChange}
          operator={operator}
          disabled={disabled}
        />
      )

    case 'date':
      return (
        <DateValueInput
          value={value}
          onChange={onChange}
          operator={operator}
          disabled={disabled}
        />
      )

    case 'similar_to': {
      const similarityValue = isSimilarityValue(value) ? value : { track_ids: [] }
      return (
        <TrackSearchInput
          selectedTrackIds={similarityValue.track_ids}
          onChange={(trackIds) => {
            const newValue: SimilarityValue = { track_ids: trackIds }
            onChange(newValue)
          }}
          disabled={disabled}
        />
      )
    }

    default: {
      // Exhaustive check - will error if new valueType added
      const _exhaustiveCheck: never = fieldConfig.valueType
      console.warn('Unhandled value type:', _exhaustiveCheck)
      return null
    }
  }
})

// ============================================================================
// String Value Input
// ============================================================================

interface StringValueInputProps {
  value: SmartRuleValue
  onChange: (value: SmartRuleValue) => void
  operator: SmartRuleOperator
  disabled: boolean
}

const StringValueInput = memo(function StringValueInput({
  value,
  onChange,
  operator,
  disabled,
}: StringValueInputProps): JSX.Element {
  // For 'in' and 'not_in' operators, use multi-value input
  if (operator === 'in' || operator === 'not_in') {
    const arrayValue = isStringArray(value) ? value : isStringValue(value) ? [value] : []
    return (
      <MultiValueInput
        values={arrayValue}
        onChange={onChange}
        disabled={disabled}
        placeholder="Type and press Enter..."
      />
    )
  }

  // Single text input
  const stringValue = isStringValue(value) ? value : ''
  return (
    <Input
      type="text"
      value={stringValue}
      onChange={(e) => onChange(e.target.value)}
      placeholder="Enter value..."
      disabled={disabled}
      className="flex-1 min-w-[120px]"
      aria-label="Rule value"
    />
  )
})

// ============================================================================
// Numeric Value Input
// ============================================================================

interface NumericValueInputProps {
  value: SmartRuleValue
  onChange: (value: SmartRuleValue) => void
  operator: SmartRuleOperator
  fieldConfig: FieldConfig
  disabled: boolean
}

const NumericValueInput = memo(function NumericValueInput({
  value,
  onChange,
  operator,
  fieldConfig,
  disabled,
}: NumericValueInputProps): JSX.Element {
  const { min = 0, max = 100, step = 1, unit } = fieldConfig

  // Memoize the slider check
  const useSlider = useMemo(
    () => fieldConfig.min === 0 && fieldConfig.max === 100 && fieldConfig.unit === '%',
    [fieldConfig.min, fieldConfig.max, fieldConfig.unit]
  )

  // For 'between' operator, show dual inputs
  if (operator === 'between') {
    const rangeValue = isRangeValue(value) ? value : null
    return (
      <div className="flex items-center gap-2">
        <Input
          type="number"
          value={rangeValue?.min ?? ''}
          onChange={(e) =>
            onChange({
              min: e.target.value ? Number(e.target.value) : min,
              max: rangeValue?.max ?? max,
            })
          }
          min={min}
          max={max}
          step={step}
          placeholder="Min"
          disabled={disabled}
          className="w-20"
          aria-label="Minimum value"
        />
        <span className="text-text-muted" aria-hidden="true">to</span>
        <Input
          type="number"
          value={rangeValue?.max ?? ''}
          onChange={(e) =>
            onChange({
              min: rangeValue?.min ?? min,
              max: e.target.value ? Number(e.target.value) : max,
            })
          }
          min={min}
          max={max}
          step={step}
          placeholder="Max"
          disabled={disabled}
          className="w-20"
          aria-label="Maximum value"
        />
        {unit && <span className="text-text-muted text-sm">{unit}</span>}
      </div>
    )
  }

  // For percentage-based values (0-100), use slider
  const numValue = isNumberValue(value) ? value : min

  if (useSlider) {
    return (
      <div className="flex items-center gap-2 flex-1 min-w-[160px]">
        <Slider
          value={numValue}
          onChange={(e) => onChange(Number(e.target.value))}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          valueFormatter={(v) => `${v}%`}
          aria-label={`${fieldConfig.label} value`}
        />
      </div>
    )
  }

  // Standard number input
  return (
    <div className="flex items-center gap-2">
      <Input
        type="number"
        value={isNumberValue(value) ? value : ''}
        onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
        min={min}
        max={max}
        step={step}
        placeholder={`${min}-${max}`}
        disabled={disabled}
        className="w-24"
        aria-label="Rule value"
      />
      {unit && <span className="text-text-muted text-sm">{unit}</span>}
    </div>
  )
})

// ============================================================================
// Array Value Input
// ============================================================================

interface ArrayValueInputProps {
  value: SmartRuleValue
  onChange: (value: SmartRuleValue) => void
  operator: SmartRuleOperator
  disabled: boolean
}

const ArrayValueInput = memo(function ArrayValueInput({
  value,
  onChange,
  operator,
  disabled,
}: ArrayValueInputProps): JSX.Element {
  // For 'contains' and 'not_contains', single value is enough
  if (operator === 'contains' || operator === 'not_contains') {
    const stringValue = isStringValue(value)
      ? value
      : isStringArray(value)
        ? value[0] || ''
        : ''
    return (
      <Input
        type="text"
        value={stringValue}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Enter value..."
        disabled={disabled}
        className="flex-1 min-w-[120px]"
        aria-label="Rule value"
      />
    )
  }

  // For 'in' and 'not_in', use multi-value input
  const arrayValue = isStringArray(value) ? value : isStringValue(value) ? [value] : []
  return (
    <MultiValueInput
      values={arrayValue}
      onChange={onChange}
      disabled={disabled}
      placeholder="Type and press Enter..."
    />
  )
})

// ============================================================================
// Date Value Input
// ============================================================================

interface DateValueInputProps {
  value: SmartRuleValue
  onChange: (value: SmartRuleValue) => void
  operator: SmartRuleOperator
  disabled: boolean
}

const DateValueInput = memo(function DateValueInput({
  value,
  onChange,
  operator,
  disabled,
}: DateValueInputProps): JSX.Element {
  // For 'between' operator, show dual date inputs
  if (operator === 'between') {
    const rangeValue = isDateRangeValue(value) ? value : null
    return (
      <div className="flex items-center gap-2">
        <Input
          type="date"
          value={rangeValue?.min ?? ''}
          onChange={(e) =>
            onChange({
              min: e.target.value,
              max: rangeValue?.max ?? '',
            })
          }
          disabled={disabled}
          className="w-36"
          aria-label="Start date"
        />
        <span className="text-text-muted" aria-hidden="true">to</span>
        <Input
          type="date"
          value={rangeValue?.max ?? ''}
          onChange={(e) =>
            onChange({
              min: rangeValue?.min ?? '',
              max: e.target.value,
            })
          }
          disabled={disabled}
          className="w-36"
          aria-label="End date"
        />
      </div>
    )
  }

  // Single date input
  const dateValue = isStringValue(value) ? value : ''
  return (
    <Input
      type="date"
      value={dateValue}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      className="w-36"
      aria-label="Date value"
    />
  )
})

// ============================================================================
// Multi-Value Input (Tags)
// ============================================================================

interface MultiValueInputProps {
  values: string[]
  onChange: (values: SmartRuleValue) => void
  disabled: boolean
  placeholder?: string
}

const MultiValueInput = memo(function MultiValueInput({
  values,
  onChange,
  disabled,
  placeholder = 'Add value...',
}: MultiValueInputProps): JSX.Element {
  const [inputValue, setInputValue] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  const addValue = useCallback(() => {
    const trimmed = inputValue.trim()
    if (trimmed && !values.includes(trimmed)) {
      onChange([...values, trimmed])
      setInputValue('')
    }
  }, [inputValue, values, onChange])

  const removeValue = useCallback(
    (index: number) => {
      const newValues = values.filter((_, i) => i !== index)
      onChange(newValues.length > 0 ? newValues : null)
      // Return focus to input after removal
      inputRef.current?.focus()
    },
    [values, onChange]
  )

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault()
        addValue()
      } else if (e.key === 'Backspace' && !inputValue && values.length > 0) {
        removeValue(values.length - 1)
      } else if (e.key === 'Escape') {
        setInputValue('')
      }
    },
    [addValue, inputValue, values.length, removeValue]
  )

  const handleBlur = useCallback(() => {
    // Commit value on blur
    addValue()
  }, [addValue])

  return (
    <div className="flex flex-col gap-2 flex-1 min-w-[160px]">
      {/* Tags display */}
      {values.length > 0 && (
        <div className="flex flex-wrap gap-1" role="list" aria-label="Selected values">
          {values.map((val, index) => (
            <span
              key={`${val}-${index}`}
              role="listitem"
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-accent/20 text-text-primary text-sm"
            >
              {val}
              <button
                type="button"
                onClick={() => removeValue(index)}
                disabled={disabled}
                className="text-text-muted hover:text-text-primary focus:outline-none focus-visible:ring-1 focus-visible:ring-accent disabled:opacity-50"
                aria-label={`Remove ${val}`}
              >
                <X size={12} />
              </button>
            </span>
          ))}
        </div>
      )}

      {/* Input */}
      <div className="flex items-center gap-2">
        <Input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          placeholder={placeholder}
          disabled={disabled}
          className="flex-1"
          aria-label="Add new value"
        />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={addValue}
          disabled={disabled || !inputValue.trim()}
        >
          Add
        </Button>
      </div>
    </div>
  )
})

// Export component types for testing
export type { SmartRuleValueInputProps }
