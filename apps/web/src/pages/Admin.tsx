/**
 * Admin Dashboard Page
 *
 * Provides admin-only functionality for user management and system monitoring.
 */

import { Users, Music, Disc, User2, Clock, HardDrive, Activity } from 'lucide-react'
import { StatCard, UsersTable } from '../components/admin'
import { useAdminSystemStats } from '../hooks/useAdmin'

export default function Admin() {
  const { data: stats, isLoading: loadingStats } = useAdminSystemStats()

  return (
    <div className="container mx-auto space-y-8 px-4 py-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold text-text-primary">Admin Dashboard</h1>
        <p className="mt-1 text-text-secondary">
          System overview and user management
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Total Users"
          value={loadingStats ? '...' : stats?.userCount ?? 0}
          icon={Users}
        />
        <StatCard
          title="Tracks"
          value={loadingStats ? '...' : stats?.trackCount.toLocaleString() ?? 0}
          icon={Music}
        />
        <StatCard
          title="Albums"
          value={loadingStats ? '...' : stats?.albumCount.toLocaleString() ?? 0}
          icon={Disc}
        />
        <StatCard
          title="Artists"
          value={loadingStats ? '...' : stats?.artistCount.toLocaleString() ?? 0}
          icon={User2}
        />
      </div>

      {/* Additional Stats */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          title="Active Sessions"
          value={loadingStats ? '...' : stats?.activeSessionCount ?? 0}
          icon={Activity}
          description="Currently logged in users"
        />
        <StatCard
          title="Library Duration"
          value={loadingStats ? '...' : stats?.totalDurationFormatted ?? '0 hours'}
          icon={Clock}
          description="Total playback time"
        />
        <StatCard
          title="Library Size"
          value={loadingStats ? '...' : stats?.totalFileSizeFormatted ?? '0 GB'}
          icon={HardDrive}
          description="Total file storage"
        />
      </div>

      {/* Users Table */}
      <UsersTable />
    </div>
  )
}
