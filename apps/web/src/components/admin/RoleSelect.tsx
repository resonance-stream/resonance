/**
 * Role selector for admin user management
 */

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/Select'
import type { UserRole } from '@resonance/shared-types'

interface RoleSelectProps {
  value: UserRole
  onChange: (value: UserRole) => void
  disabled?: boolean
}

const ROLES: { value: UserRole; label: string; description: string }[] = [
  { value: 'admin', label: 'Admin', description: 'Full system access' },
  { value: 'user', label: 'User', description: 'Standard user access' },
  { value: 'guest', label: 'Guest', description: 'Limited access' },
]

export function RoleSelect({ value, onChange, disabled }: RoleSelectProps) {
  return (
    <Select value={value} onValueChange={onChange as (value: string) => void} disabled={disabled}>
      <SelectTrigger className="w-32">
        <SelectValue placeholder="Select role" />
      </SelectTrigger>
      <SelectContent>
        {ROLES.map((role) => (
          <SelectItem key={role.value} value={role.value}>
            <span className="flex items-center gap-2">
              <span>{role.label}</span>
            </span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
