/**
 * User detail modal for admin management
 */

import { useState } from 'react'
import { Monitor, Smartphone, Tablet, Globe, LogOut, Trash2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../ui/Dialog'
import { Button } from '../ui/Button'
import { RoleSelect } from './RoleSelect'
import { useAdminUser, useUpdateUserRole, useDeleteUser, useInvalidateSessions } from '../../hooks/useAdmin'
import type { UserRole } from '@resonance/shared-types'

interface UserDetailModalProps {
  userId: string | null
  onClose: () => void
}

function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function getDeviceIcon(deviceType?: string | null) {
  switch (deviceType?.toLowerCase()) {
    case 'mobile':
      return <Smartphone className="h-4 w-4" />
    case 'tablet':
      return <Tablet className="h-4 w-4" />
    case 'desktop':
      return <Monitor className="h-4 w-4" />
    default:
      return <Globe className="h-4 w-4" />
  }
}

export function UserDetailModal({ userId, onClose }: UserDetailModalProps) {
  const [confirmDelete, setConfirmDelete] = useState(false)

  const { data: userDetail, isLoading } = useAdminUser(userId || '', {
    enabled: !!userId,
  })

  const updateRole = useUpdateUserRole()
  const deleteUser = useDeleteUser()
  const invalidateSessions = useInvalidateSessions()

  const handleRoleChange = async (newRole: UserRole) => {
    if (!userId) return
    try {
      await updateRole.mutateAsync({ userId, role: newRole })
    } catch (error) {
      console.error('Failed to update role:', error)
    }
  }

  const handleInvalidateSessions = async () => {
    if (!userId) return
    try {
      await invalidateSessions.mutateAsync(userId)
    } catch (error) {
      console.error('Failed to invalidate sessions:', error)
    }
  }

  const handleDeleteUser = async () => {
    if (!userId) return
    try {
      await deleteUser.mutateAsync(userId)
      onClose()
    } catch (error) {
      console.error('Failed to delete user:', error)
    }
  }

  return (
    <Dialog open={!!userId} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>User Details</DialogTitle>
          <DialogDescription>
            View and manage user account settings
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-8 text-center text-text-secondary">Loading...</div>
        ) : userDetail ? (
          <div className="space-y-6">
            {/* User Info */}
            <div className="flex items-start gap-4">
              {userDetail.user.avatarUrl ? (
                <img
                  src={userDetail.user.avatarUrl}
                  alt=""
                  className="h-12 w-12 rounded-full object-cover"
                />
              ) : (
                <div className="flex h-12 w-12 items-center justify-center rounded-full bg-accent-dark/30 text-lg font-medium text-accent">
                  {userDetail.user.displayName.charAt(0).toUpperCase()}
                </div>
              )}
              <div className="flex-1">
                <h3 className="font-medium text-text-primary">
                  {userDetail.user.displayName}
                </h3>
                <p className="text-sm text-text-secondary">
                  {userDetail.user.email}
                </p>
                <p className="mt-1 text-xs text-text-tertiary">
                  Joined {formatDate(userDetail.user.createdAt)}
                </p>
              </div>
            </div>

            {/* Role */}
            <div className="flex items-center justify-between rounded-lg bg-background-tertiary/50 p-3">
              <span className="text-sm text-text-secondary">Role</span>
              <RoleSelect
                value={userDetail.user.role}
                onChange={handleRoleChange}
                disabled={updateRole.isPending}
              />
            </div>

            {/* Status */}
            <div className="flex items-center justify-between rounded-lg bg-background-tertiary/50 p-3">
              <span className="text-sm text-text-secondary">Email Verified</span>
              <span className={`text-sm ${userDetail.user.emailVerified ? 'text-green-400' : 'text-yellow-400'}`}>
                {userDetail.user.emailVerified ? 'Yes' : 'No'}
              </span>
            </div>

            {/* Sessions */}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <h4 className="text-sm font-medium text-text-primary">
                  Sessions ({userDetail.sessions.filter(s => s.isActive).length} active)
                </h4>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleInvalidateSessions}
                  disabled={invalidateSessions.isPending}
                >
                  <LogOut className="mr-1 h-3 w-3" />
                  Logout All
                </Button>
              </div>
              <div className="max-h-48 space-y-2 overflow-y-auto">
                {userDetail.sessions.length === 0 ? (
                  <p className="py-4 text-center text-sm text-text-tertiary">
                    No sessions found
                  </p>
                ) : (
                  userDetail.sessions.map((session) => (
                    <div
                      key={session.id}
                      className={`flex items-center gap-3 rounded-lg p-2 ${
                        session.isActive
                          ? 'bg-green-500/10 border border-green-500/20'
                          : 'bg-background-tertiary/30'
                      }`}
                    >
                      {getDeviceIcon(session.deviceType)}
                      <div className="flex-1 min-w-0">
                        <p className="text-sm text-text-primary truncate">
                          {session.deviceName || session.deviceType || 'Unknown Device'}
                        </p>
                        <p className="text-xs text-text-tertiary">
                          {session.ipAddress || 'Unknown IP'} &bull; {formatDate(session.lastActiveAt)}
                        </p>
                      </div>
                      {session.isActive && (
                        <span className="text-xs text-green-400">Active</span>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        ) : null}

        <DialogFooter>
          {confirmDelete ? (
            <>
              <span className="mr-auto text-sm text-red-400">
                Are you sure?
              </span>
              <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                className="bg-red-600 hover:bg-red-700"
                onClick={handleDeleteUser}
                disabled={deleteUser.isPending}
              >
                Delete
              </Button>
            </>
          ) : (
            <>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setConfirmDelete(true)}
                className="text-red-400 hover:text-red-300"
              >
                <Trash2 className="mr-1 h-3 w-3" />
                Delete User
              </Button>
              <Button variant="secondary" size="sm" onClick={onClose}>
                Close
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
