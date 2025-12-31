/**
 * Users table for admin management
 */

import { useState, useCallback } from 'react'
import { Search, ChevronLeft, ChevronRight, Shield, User as UserIcon, Eye } from 'lucide-react'
import { Card } from '../ui/Card'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Badge } from '../ui/Badge'
import { useAdminUsers } from '../../hooks/useAdmin'
import { UserDetailModal } from './UserDetailModal'
import type { GqlAdminUserListItem } from '../../types/admin'

const PAGE_SIZE = 10

function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  })
}

function getRoleBadge(role: string) {
  switch (role.toLowerCase()) {
    case 'admin':
      return (
        <Badge variant="info" className="gap-1">
          <Shield className="h-3 w-3" />
          Admin
        </Badge>
      )
    case 'guest':
      return <Badge variant="warning">Guest</Badge>
    default:
      return (
        <Badge variant="default" className="gap-1">
          <UserIcon className="h-3 w-3" />
          User
        </Badge>
      )
  }
}

export function UsersTable() {
  const [page, setPage] = useState(0)
  const [search, setSearch] = useState('')
  const [searchDebounced, setSearchDebounced] = useState('')
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null)

  // Debounce search input
  const handleSearchChange = useCallback((value: string) => {
    setSearch(value)
    // Simple debounce using setTimeout
    const timeoutId = setTimeout(() => {
      setSearchDebounced(value)
      setPage(0) // Reset to first page on search
    }, 300)
    return () => clearTimeout(timeoutId)
  }, [])

  const { data, isLoading, error } = useAdminUsers({
    limit: PAGE_SIZE,
    offset: page * PAGE_SIZE,
    search: searchDebounced || undefined,
  })

  const totalPages = data ? Math.ceil(data.totalCount / PAGE_SIZE) : 0

  const handleViewUser = (user: GqlAdminUserListItem) => {
    setSelectedUserId(user.id)
  }

  if (error) {
    return (
      <Card variant="glass" padding="lg">
        <div className="text-center text-red-400">
          Failed to load users. {error.message}
        </div>
      </Card>
    )
  }

  return (
    <>
      <Card variant="glass" padding="none">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-white/5 p-4">
          <h2 className="text-lg font-semibold text-text-primary">Users</h2>
          <div className="relative w-64">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-tertiary" />
            <Input
              placeholder="Search users..."
              value={search}
              onChange={(e) => handleSearchChange(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>

        {/* Table */}
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-white/5 text-left text-sm text-text-secondary">
                <th className="px-4 py-3 font-medium">User</th>
                <th className="px-4 py-3 font-medium">Role</th>
                <th className="px-4 py-3 font-medium">Sessions</th>
                <th className="px-4 py-3 font-medium">Joined</th>
                <th className="px-4 py-3 font-medium">Last Seen</th>
                <th className="px-4 py-3 font-medium"></th>
              </tr>
            </thead>
            <tbody>
              {isLoading ? (
                // Loading skeleton
                [...Array(5)].map((_, i) => (
                  <tr key={i} className="border-b border-white/5">
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 animate-pulse rounded-full bg-background-tertiary" />
                        <div className="space-y-1">
                          <div className="h-4 w-24 animate-pulse rounded bg-background-tertiary" />
                          <div className="h-3 w-32 animate-pulse rounded bg-background-tertiary" />
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-4">
                      <div className="h-5 w-16 animate-pulse rounded bg-background-tertiary" />
                    </td>
                    <td className="px-4 py-4">
                      <div className="h-4 w-8 animate-pulse rounded bg-background-tertiary" />
                    </td>
                    <td className="px-4 py-4">
                      <div className="h-4 w-20 animate-pulse rounded bg-background-tertiary" />
                    </td>
                    <td className="px-4 py-4">
                      <div className="h-4 w-20 animate-pulse rounded bg-background-tertiary" />
                    </td>
                    <td className="px-4 py-4"></td>
                  </tr>
                ))
              ) : data?.users.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-text-secondary">
                    {searchDebounced ? 'No users found matching your search.' : 'No users found.'}
                  </td>
                </tr>
              ) : (
                data?.users.map((user) => (
                  <tr
                    key={user.id}
                    className="border-b border-white/5 hover:bg-background-tertiary/30 transition-colors"
                  >
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-3">
                        {user.avatarUrl ? (
                          <img
                            src={user.avatarUrl}
                            alt=""
                            className="h-8 w-8 rounded-full object-cover"
                          />
                        ) : (
                          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-accent-dark/30 text-sm font-medium text-accent">
                            {user.displayName.charAt(0).toUpperCase()}
                          </div>
                        )}
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-medium text-text-primary">
                              {user.displayName}
                            </span>
                            {user.emailVerified && (
                              <span className="text-xs text-green-400" title="Email verified">
                                ✓
                              </span>
                            )}
                          </div>
                          <span className="text-sm text-text-tertiary">
                            {user.email}
                          </span>
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-4">{getRoleBadge(user.role)}</td>
                    <td className="px-4 py-4">
                      <span className={user.sessionCount > 0 ? 'text-green-400' : 'text-text-tertiary'}>
                        {user.sessionCount}
                      </span>
                    </td>
                    <td className="px-4 py-4 text-sm text-text-secondary">
                      {formatDate(user.createdAt)}
                    </td>
                    <td className="px-4 py-4 text-sm text-text-secondary">
                      {user.lastSeenAt ? formatDate(user.lastSeenAt) : 'Never'}
                    </td>
                    <td className="px-4 py-4">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleViewUser(user)}
                        title="View details"
                      >
                        <Eye className="h-4 w-4" />
                      </Button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        {data && data.totalCount > PAGE_SIZE && (
          <div className="flex items-center justify-between border-t border-white/5 px-4 py-3">
            <span className="text-sm text-text-secondary">
              Showing {page * PAGE_SIZE + 1}-{Math.min((page + 1) * PAGE_SIZE, data.totalCount)} of {data.totalCount}
            </span>
            <div className="flex gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={page === 0}
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <span className="flex items-center px-2 text-sm text-text-secondary">
                Page {page + 1} of {totalPages}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                disabled={!data.hasNextPage}
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </Card>

      <UserDetailModal
        userId={selectedUserId}
        onClose={() => setSelectedUserId(null)}
      />
    </>
  )
}
