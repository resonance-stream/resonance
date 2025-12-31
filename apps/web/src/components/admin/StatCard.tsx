/**
 * Stat card for displaying admin dashboard statistics
 */

import { Card } from '../ui/Card'
import type { LucideIcon } from 'lucide-react'

interface StatCardProps {
  title: string
  value: string | number
  icon: LucideIcon
  description?: string
}

export function StatCard({ title, value, icon: Icon, description }: StatCardProps) {
  return (
    <Card variant="glass" padding="lg">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-text-secondary">{title}</p>
          <p className="mt-1 text-2xl font-bold text-text-primary">{value}</p>
          {description && (
            <p className="mt-1 text-xs text-text-tertiary">{description}</p>
          )}
        </div>
        <div className="rounded-lg bg-accent-dark/20 p-2">
          <Icon className="h-5 w-5 text-accent" />
        </div>
      </div>
    </Card>
  )
}
