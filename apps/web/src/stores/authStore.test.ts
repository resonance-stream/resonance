/**
 * Auth Store Tests
 *
 * Comprehensive tests for the Zustand auth store covering:
 * - Login success/failure
 * - Register success/failure
 * - Token refresh
 * - Logout
 * - Hydration
 * - Error parsing
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { useAuthStore } from './authStore'

// Mock the api module
const mockSetAuthToken = vi.fn()
const mockGraphqlRequest = vi.fn()

vi.mock('../lib/api', () => ({
  graphqlClient: {
    request: (...args: unknown[]) => mockGraphqlRequest(...args),
  },
  setAuthToken: (...args: unknown[]) => mockSetAuthToken(...args),
}))

// Helper to reset the store between tests
function resetStore(): void {
  useAuthStore.setState({
    user: null,
    accessToken: null,
    refreshToken: null,
    expiresAt: null,
    status: 'idle',
    error: null,
  })
}

// Mock auth response
const mockAuthPayload = {
  id: 'user-123',
  email: 'test@example.com',
  displayName: 'Test User',
  avatarUrl: null,
  role: 'USER',
  emailVerified: true,
  accessToken: 'mock-access-token',
  refreshToken: 'mock-refresh-token',
  expiresAt: new Date(Date.now() + 3600 * 1000).toISOString(), // 1 hour from now
  tokenType: 'Bearer',
}

const mockUser = {
  id: 'user-123',
  username: 'test',
  email: 'test@example.com',
  displayName: 'Test User',
  avatarUrl: undefined,
  role: 'user' as const,
  emailVerified: true,
  createdAt: expect.any(String),
  updatedAt: expect.any(String),
}

describe('authStore', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('initial state', () => {
    it('has null user', () => {
      expect(useAuthStore.getState().user).toBeNull()
    })

    it('has null tokens', () => {
      expect(useAuthStore.getState().accessToken).toBeNull()
      expect(useAuthStore.getState().refreshToken).toBeNull()
    })

    it('has idle status', () => {
      expect(useAuthStore.getState().status).toBe('idle')
    })

    it('has null error', () => {
      expect(useAuthStore.getState().error).toBeNull()
    })
  })

  describe('login', () => {
    it('successfully logs in with valid credentials', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({
        login: mockAuthPayload,
      })

      await useAuthStore.getState().login({
        email: 'test@example.com',
        password: 'password123',
      })

      const state = useAuthStore.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).toMatchObject({
        id: 'user-123',
        email: 'test@example.com',
        displayName: 'Test User',
      })
      expect(state.accessToken).toBe('mock-access-token')
      expect(state.refreshToken).toBe('mock-refresh-token')
      expect(state.error).toBeNull()
      expect(mockSetAuthToken).toHaveBeenCalledWith('mock-access-token')
    })

    it('sets loading status during login', async () => {
      let statusDuringRequest: string | undefined

      mockGraphqlRequest.mockImplementationOnce(async () => {
        statusDuringRequest = useAuthStore.getState().status
        return { login: mockAuthPayload }
      })

      await useAuthStore.getState().login({
        email: 'test@example.com',
        password: 'password123',
      })

      expect(statusDuringRequest).toBe('loading')
    })

    it('handles invalid credentials error', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Invalid credentials'))

      await expect(
        useAuthStore.getState().login({
          email: 'test@example.com',
          password: 'wrongpassword',
        })
      ).rejects.toMatchObject({
        code: 'INVALID_CREDENTIALS',
      })

      const state = useAuthStore.getState()
      expect(state.status).toBe('unauthenticated')
      expect(state.error).toMatchObject({
        code: 'INVALID_CREDENTIALS',
        message: 'Invalid email or password',
      })
      expect(state.user).toBeNull()
    })

    it('handles wrong password error', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Wrong password'))

      await expect(
        useAuthStore.getState().login({
          email: 'test@example.com',
          password: 'wrongpassword',
        })
      ).rejects.toMatchObject({
        code: 'INVALID_CREDENTIALS',
      })

      const state = useAuthStore.getState()
      expect(state.error?.code).toBe('INVALID_CREDENTIALS')
    })

    it('handles user not found error', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('User not found'))

      await expect(
        useAuthStore.getState().login({
          email: 'nonexistent@example.com',
          password: 'password123',
        })
      ).rejects.toMatchObject({
        code: 'USER_NOT_FOUND',
      })

      const state = useAuthStore.getState()
      expect(state.error?.code).toBe('USER_NOT_FOUND')
    })

    it('handles network errors', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Failed to fetch'))

      await expect(
        useAuthStore.getState().login({
          email: 'test@example.com',
          password: 'password123',
        })
      ).rejects.toMatchObject({
        code: 'NETWORK_ERROR',
      })

      const state = useAuthStore.getState()
      expect(state.status).toBe('unauthenticated')
      expect(state.error?.code).toBe('NETWORK_ERROR')
    })

    it('clears previous errors before login attempt', async () => {
      // Set initial error state
      useAuthStore.setState({
        error: { code: 'UNKNOWN_ERROR', message: 'Previous error' },
      })

      mockGraphqlRequest.mockResolvedValueOnce({
        login: mockAuthPayload,
      })

      await useAuthStore.getState().login({
        email: 'test@example.com',
        password: 'password123',
      })

      expect(useAuthStore.getState().error).toBeNull()
    })

    it('parses expiresAt with 60 second buffer', async () => {
      const expiresAt = new Date(Date.now() + 3600 * 1000).toISOString()
      const expectedExpiresAtMs = new Date(expiresAt).getTime() - 60 * 1000

      mockGraphqlRequest.mockResolvedValueOnce({
        login: {
          ...mockAuthPayload,
          expiresAt,
        },
      })

      await useAuthStore.getState().login({
        email: 'test@example.com',
        password: 'password',
      })

      expect(useAuthStore.getState().expiresAt).toBe(expectedExpiresAtMs)
    })

    it('sends correct GraphQL variables', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({
        login: mockAuthPayload,
      })

      await useAuthStore.getState().login({
        email: 'test@example.com',
        password: 'mypassword',
      })

      expect(mockGraphqlRequest).toHaveBeenCalledWith(
        expect.anything(),
        {
          input: {
            email: 'test@example.com',
            password: 'mypassword',
          },
        }
      )
    })
  })

  describe('register', () => {
    it('successfully registers a new user', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({
        register: mockAuthPayload,
      })

      await useAuthStore.getState().register({
        email: 'newuser@example.com',
        password: 'password123',
        displayName: 'New User',
      })

      const state = useAuthStore.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).not.toBeNull()
      expect(state.accessToken).toBe('mock-access-token')
      expect(mockSetAuthToken).toHaveBeenCalledWith('mock-access-token')
    })

    it('uses email prefix as displayName when not provided', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({
        register: mockAuthPayload,
      })

      await useAuthStore.getState().register({
        email: 'newuser@example.com',
        password: 'password123',
      })

      expect(mockGraphqlRequest).toHaveBeenCalledWith(
        expect.anything(),
        {
          input: {
            email: 'newuser@example.com',
            password: 'password123',
            display_name: 'newuser',
          },
        }
      )
    })

    it('uses provided displayName when given', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({
        register: mockAuthPayload,
      })

      await useAuthStore.getState().register({
        email: 'newuser@example.com',
        password: 'password123',
        displayName: 'My Custom Name',
      })

      expect(mockGraphqlRequest).toHaveBeenCalledWith(
        expect.anything(),
        {
          input: {
            email: 'newuser@example.com',
            password: 'password123',
            display_name: 'My Custom Name',
          },
        }
      )
    })

    it('handles email already exists error', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Email already exists'))

      await expect(
        useAuthStore.getState().register({
          email: 'existing@example.com',
          password: 'password123',
        })
      ).rejects.toMatchObject({
        code: 'EMAIL_ALREADY_EXISTS',
      })

      const state = useAuthStore.getState()
      expect(state.error?.code).toBe('EMAIL_ALREADY_EXISTS')
      expect(state.status).toBe('unauthenticated')
    })

    it('handles username already exists error', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Username already exists'))

      await expect(
        useAuthStore.getState().register({
          email: 'test@example.com',
          password: 'password123',
        })
      ).rejects.toMatchObject({
        code: 'USERNAME_ALREADY_EXISTS',
      })
    })

    it('sets loading status during registration', async () => {
      let statusDuringRequest: string | undefined

      mockGraphqlRequest.mockImplementationOnce(async () => {
        statusDuringRequest = useAuthStore.getState().status
        return { register: mockAuthPayload }
      })

      await useAuthStore.getState().register({
        email: 'test@example.com',
        password: 'password123',
      })

      expect(statusDuringRequest).toBe('loading')
    })

    it('clears previous errors before registration', async () => {
      useAuthStore.setState({
        error: { code: 'UNKNOWN_ERROR', message: 'Previous error' },
      })

      mockGraphqlRequest.mockResolvedValueOnce({
        register: mockAuthPayload,
      })

      await useAuthStore.getState().register({
        email: 'test@example.com',
        password: 'password123',
      })

      expect(useAuthStore.getState().error).toBeNull()
    })
  })

  describe('logout', () => {
    beforeEach(() => {
      // Set up authenticated state
      useAuthStore.setState({
        user: mockUser,
        accessToken: 'mock-access-token',
        refreshToken: 'mock-refresh-token',
        expiresAt: Date.now() + 3600 * 1000,
        status: 'authenticated',
        error: null,
      })
    })

    it('clears all auth state on logout', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({ logout: true })

      await useAuthStore.getState().logout()

      const state = useAuthStore.getState()
      expect(state.user).toBeNull()
      expect(state.accessToken).toBeNull()
      expect(state.refreshToken).toBeNull()
      expect(state.expiresAt).toBeNull()
      expect(state.status).toBe('unauthenticated')
      expect(state.error).toBeNull()
    })

    it('clears auth token header on logout', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({ logout: true })

      await useAuthStore.getState().logout()

      expect(mockSetAuthToken).toHaveBeenCalledWith(null)
    })

    it('clears state even if logout mutation fails', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Network error'))

      await useAuthStore.getState().logout()

      const state = useAuthStore.getState()
      expect(state.user).toBeNull()
      expect(state.accessToken).toBeNull()
      expect(state.status).toBe('unauthenticated')
    })

    it('does not call logout mutation if no access token', async () => {
      useAuthStore.setState({ accessToken: null })

      await useAuthStore.getState().logout()

      expect(mockGraphqlRequest).not.toHaveBeenCalled()
      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })

    it('calls logout mutation with no variables', async () => {
      mockGraphqlRequest.mockResolvedValueOnce({ logout: true })

      await useAuthStore.getState().logout()

      expect(mockGraphqlRequest).toHaveBeenCalledWith(expect.anything())
    })
  })

  describe('refreshAccessToken', () => {
    beforeEach(() => {
      useAuthStore.setState({
        user: mockUser,
        accessToken: 'old-access-token',
        refreshToken: 'mock-refresh-token',
        expiresAt: Date.now() + 3600 * 1000,
        status: 'authenticated',
        error: null,
      })
    })

    it('successfully refreshes the access token', async () => {
      const newExpiresAt = new Date(Date.now() + 7200 * 1000).toISOString()

      mockGraphqlRequest.mockResolvedValueOnce({
        refreshToken: {
          accessToken: 'new-access-token',
          refreshToken: 'new-refresh-token',
          expiresAt: newExpiresAt,
          tokenType: 'Bearer',
        },
      })

      const result = await useAuthStore.getState().refreshAccessToken()

      expect(result).toBe(true)
      const state = useAuthStore.getState()
      expect(state.accessToken).toBe('new-access-token')
      expect(state.refreshToken).toBe('new-refresh-token')
      expect(state.status).toBe('authenticated')
      expect(mockSetAuthToken).toHaveBeenCalledWith('new-access-token')
    })

    it('returns false if no refresh token exists', async () => {
      useAuthStore.setState({ refreshToken: null })

      const result = await useAuthStore.getState().refreshAccessToken()

      expect(result).toBe(false)
      expect(useAuthStore.getState().status).toBe('unauthenticated')
      expect(mockGraphqlRequest).not.toHaveBeenCalled()
    })

    it('clears session and returns false on refresh failure', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Token invalid'))

      const result = await useAuthStore.getState().refreshAccessToken()

      expect(result).toBe(false)
      const state = useAuthStore.getState()
      expect(state.user).toBeNull()
      expect(state.accessToken).toBeNull()
      expect(state.refreshToken).toBeNull()
      expect(state.status).toBe('unauthenticated')
      expect(state.error?.code).toBe('SESSION_EXPIRED')
    })

    it('clears auth header on refresh failure', async () => {
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Token expired'))

      await useAuthStore.getState().refreshAccessToken()

      expect(mockSetAuthToken).toHaveBeenCalledWith(null)
    })

    it('passes refresh token as mutation variable', async () => {
      const newExpiresAt = new Date(Date.now() + 7200 * 1000).toISOString()

      mockGraphqlRequest.mockResolvedValueOnce({
        refreshToken: {
          accessToken: 'new-access-token',
          refreshToken: 'new-refresh-token',
          expiresAt: newExpiresAt,
          tokenType: 'Bearer',
        },
      })

      await useAuthStore.getState().refreshAccessToken()

      expect(mockGraphqlRequest).toHaveBeenCalledWith(
        expect.anything(),
        { refreshToken: 'mock-refresh-token' }
      )
    })

    it('applies 60 second buffer to new expiry', async () => {
      const newExpiresAt = new Date(Date.now() + 7200 * 1000).toISOString()
      const expectedExpiresAtMs = new Date(newExpiresAt).getTime() - 60 * 1000

      mockGraphqlRequest.mockResolvedValueOnce({
        refreshToken: {
          accessToken: 'new-access-token',
          refreshToken: 'new-refresh-token',
          expiresAt: newExpiresAt,
          tokenType: 'Bearer',
        },
      })

      await useAuthStore.getState().refreshAccessToken()

      expect(useAuthStore.getState().expiresAt).toBe(expectedExpiresAtMs)
    })
  })

  describe('fetchCurrentUser', () => {
    it('fetches and sets current user when authenticated', async () => {
      useAuthStore.setState({
        accessToken: 'mock-access-token',
        status: 'authenticated',
      })

      const mockMeResponse = {
        id: 'user-123',
        email: 'test@example.com',
        displayName: 'Test User',
        avatarUrl: null,
        role: 'USER',
        emailVerified: true,
        lastSeenAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      }

      mockGraphqlRequest.mockResolvedValueOnce({
        me: mockMeResponse,
      })

      await useAuthStore.getState().fetchCurrentUser()

      const state = useAuthStore.getState()
      expect(state.status).toBe('authenticated')
      expect(state.user).toMatchObject({
        id: 'user-123',
        email: 'test@example.com',
      })
    })

    it('sets unauthenticated if no access token', async () => {
      useAuthStore.setState({ accessToken: null })

      await useAuthStore.getState().fetchCurrentUser()

      expect(useAuthStore.getState().status).toBe('unauthenticated')
      expect(mockGraphqlRequest).not.toHaveBeenCalled()
    })

    it('sets loading status during fetch', async () => {
      useAuthStore.setState({
        accessToken: 'mock-access-token',
        status: 'authenticated',
      })

      let statusDuringRequest: string | undefined

      mockGraphqlRequest.mockImplementationOnce(async () => {
        statusDuringRequest = useAuthStore.getState().status
        return {
          me: {
            id: 'user-123',
            email: 'test@example.com',
            displayName: 'Test User',
            avatarUrl: null,
            role: 'USER',
            emailVerified: true,
          },
        }
      })

      await useAuthStore.getState().fetchCurrentUser()

      expect(statusDuringRequest).toBe('loading')
    })

    it('attempts token refresh on fetch failure', async () => {
      useAuthStore.setState({
        accessToken: 'expired-access-token',
        refreshToken: 'mock-refresh-token',
        status: 'authenticated',
      })

      const newExpiresAt = new Date(Date.now() + 3600 * 1000).toISOString()

      // First call fails (me query)
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Token expired'))
      // Second call succeeds (refresh mutation)
      mockGraphqlRequest.mockResolvedValueOnce({
        refreshToken: {
          accessToken: 'new-access-token',
          refreshToken: 'new-refresh-token',
          expiresAt: newExpiresAt,
          tokenType: 'Bearer',
        },
      })

      await useAuthStore.getState().fetchCurrentUser()

      // Should have attempted refresh
      expect(useAuthStore.getState().accessToken).toBe('new-access-token')
    })

    it('sets unauthenticated if refresh also fails', async () => {
      useAuthStore.setState({
        accessToken: 'expired-access-token',
        refreshToken: 'mock-refresh-token',
        status: 'authenticated',
      })

      // Both calls fail
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Token expired'))
      mockGraphqlRequest.mockRejectedValueOnce(new Error('Refresh failed'))

      await useAuthStore.getState().fetchCurrentUser()

      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })

    it('sets auth header before fetching', async () => {
      useAuthStore.setState({
        accessToken: 'mock-access-token',
        status: 'authenticated',
      })

      mockGraphqlRequest.mockResolvedValueOnce({
        me: {
          id: 'user-123',
          email: 'test@example.com',
        },
      })

      await useAuthStore.getState().fetchCurrentUser()

      expect(mockSetAuthToken).toHaveBeenCalledWith('mock-access-token')
    })
  })

  describe('hydrate', () => {
    it('sets authenticated status for valid non-expired token', () => {
      const futureExpiry = Date.now() + 3600 * 1000 // 1 hour from now

      useAuthStore.setState({
        accessToken: 'valid-token',
        refreshToken: 'refresh-token',
        expiresAt: futureExpiry,
        status: 'idle',
      })

      useAuthStore.getState().hydrate()

      expect(useAuthStore.getState().status).toBe('authenticated')
      expect(mockSetAuthToken).toHaveBeenCalledWith('valid-token')
    })

    it('attempts refresh for expired token', async () => {
      const pastExpiry = Date.now() - 3600 * 1000 // 1 hour ago
      const newExpiresAt = new Date(Date.now() + 3600 * 1000).toISOString()

      mockGraphqlRequest.mockResolvedValueOnce({
        refreshToken: {
          accessToken: 'new-access-token',
          refreshToken: 'new-refresh-token',
          expiresAt: newExpiresAt,
          tokenType: 'Bearer',
        },
      })

      useAuthStore.setState({
        accessToken: 'expired-token',
        refreshToken: 'refresh-token',
        expiresAt: pastExpiry,
        status: 'idle',
      })

      useAuthStore.getState().hydrate()

      // Wait for async refresh to complete
      await vi.waitFor(() => {
        expect(useAuthStore.getState().accessToken).toBe('new-access-token')
      })
    })

    it('sets unauthenticated when no tokens exist', () => {
      useAuthStore.setState({
        accessToken: null,
        refreshToken: null,
        expiresAt: null,
        status: 'idle',
      })

      useAuthStore.getState().hydrate()

      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })

    it('sets unauthenticated when token exists but no expiry', () => {
      useAuthStore.setState({
        accessToken: 'some-token',
        refreshToken: 'refresh-token',
        expiresAt: null,
        status: 'idle',
      })

      useAuthStore.getState().hydrate()

      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })

    it('sets unauthenticated when expiry exists but no token', () => {
      useAuthStore.setState({
        accessToken: null,
        refreshToken: 'refresh-token',
        expiresAt: Date.now() + 3600 * 1000,
        status: 'idle',
      })

      useAuthStore.getState().hydrate()

      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })
  })

  describe('clearError', () => {
    it('clears the error state', () => {
      useAuthStore.setState({
        error: { code: 'UNKNOWN_ERROR', message: 'Some error' },
      })

      useAuthStore.getState().clearError()

      expect(useAuthStore.getState().error).toBeNull()
    })

    it('does nothing if no error exists', () => {
      useAuthStore.setState({ error: null })

      useAuthStore.getState().clearError()

      expect(useAuthStore.getState().error).toBeNull()
    })
  })

  describe('setStatus', () => {
    it('sets the auth status to loading', () => {
      useAuthStore.getState().setStatus('loading')
      expect(useAuthStore.getState().status).toBe('loading')
    })

    it('sets the auth status to authenticated', () => {
      useAuthStore.getState().setStatus('authenticated')
      expect(useAuthStore.getState().status).toBe('authenticated')
    })

    it('sets the auth status to unauthenticated', () => {
      useAuthStore.getState().setStatus('unauthenticated')
      expect(useAuthStore.getState().status).toBe('unauthenticated')
    })

    it('sets the auth status to idle', () => {
      useAuthStore.getState().setStatus('idle')
      expect(useAuthStore.getState().status).toBe('idle')
    })
  })
})

describe('parseAuthError (integration tests)', () => {
  // We test error parsing through the login/register flows since parseAuthError is not exported

  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  it('parses invalid credentials error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Invalid credentials provided'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'wrong' })
    ).rejects.toMatchObject({ code: 'INVALID_CREDENTIALS' })
  })

  it('parses wrong password error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Wrong password'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'wrong' })
    ).rejects.toMatchObject({ code: 'INVALID_CREDENTIALS' })
  })

  it('parses user not found error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('User not found in database'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'USER_NOT_FOUND' })
  })

  it('parses email exists error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Email already exists'))

    await expect(
      useAuthStore.getState().register({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'EMAIL_ALREADY_EXISTS' })
  })

  it('parses username exists error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Username already exists'))

    await expect(
      useAuthStore.getState().register({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'USERNAME_ALREADY_EXISTS' })
  })

  it('parses token expired error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Token has expired'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'TOKEN_EXPIRED' })
  })

  it('parses token invalid error', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Token is invalid'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'TOKEN_INVALID' })
  })

  it('parses network errors with fetch keyword', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Failed to fetch'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'NETWORK_ERROR' })
  })

  it('parses network errors with network keyword', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Network request failed'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'NETWORK_ERROR' })
  })

  it('returns UNKNOWN_ERROR for unrecognized errors', async () => {
    mockGraphqlRequest.mockRejectedValueOnce(new Error('Some random error that does not match patterns'))

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({ code: 'UNKNOWN_ERROR' })
  })

  it('handles non-Error objects gracefully', async () => {
    mockGraphqlRequest.mockRejectedValueOnce('string error')

    await expect(
      useAuthStore.getState().login({ email: 'test@example.com', password: 'pass' })
    ).rejects.toMatchObject({
      code: 'UNKNOWN_ERROR',
      message: 'An unexpected error occurred',
    })
  })
})

describe('extractUserFromPayload (integration tests)', () => {
  // We test user extraction through login/register since extractUserFromPayload is not exported

  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  it('converts USER role to lowercase', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        role: 'USER',
      },
    })

    await useAuthStore.getState().login({
      email: 'test@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.role).toBe('user')
  })

  it('converts ADMIN role to lowercase', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        role: 'ADMIN',
      },
    })

    await useAuthStore.getState().login({
      email: 'admin@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.role).toBe('admin')
  })

  it('converts GUEST role to lowercase', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        role: 'GUEST',
      },
    })

    await useAuthStore.getState().login({
      email: 'guest@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.role).toBe('guest')
  })

  it('extracts username from email prefix', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        email: 'myusername@example.com',
      },
    })

    await useAuthStore.getState().login({
      email: 'myusername@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.username).toBe('myusername')
  })

  it('handles email without @ symbol', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        email: 'localuser',
      },
    })

    await useAuthStore.getState().login({
      email: 'localuser',
      password: 'password',
    })

    // When no @ symbol, split returns the whole string
    expect(useAuthStore.getState().user?.username).toBe('localuser')
  })

  it('converts null avatarUrl to undefined', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        avatarUrl: null,
      },
    })

    await useAuthStore.getState().login({
      email: 'test@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.avatarUrl).toBeUndefined()
  })

  it('preserves string avatarUrl', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        ...mockAuthPayload,
        avatarUrl: 'https://example.com/avatar.jpg',
      },
    })

    await useAuthStore.getState().login({
      email: 'test@example.com',
      password: 'password',
    })

    expect(useAuthStore.getState().user?.avatarUrl).toBe('https://example.com/avatar.jpg')
  })

  it('sets createdAt and updatedAt to current time', async () => {
    const beforeTime = new Date().toISOString()

    mockGraphqlRequest.mockResolvedValueOnce({
      login: mockAuthPayload,
    })

    await useAuthStore.getState().login({
      email: 'test@example.com',
      password: 'password',
    })

    const afterTime = new Date().toISOString()
    const user = useAuthStore.getState().user

    expect(user?.createdAt).toBeDefined()
    expect(user?.updatedAt).toBeDefined()
    // The timestamps should be between beforeTime and afterTime
    expect(user?.createdAt! >= beforeTime).toBe(true)
    expect(user?.createdAt! <= afterTime).toBe(true)
  })

  it('correctly maps all user fields', async () => {
    mockGraphqlRequest.mockResolvedValueOnce({
      login: {
        id: 'test-id-456',
        email: 'fulltest@example.com',
        displayName: 'Full Test User',
        avatarUrl: 'https://example.com/img.png',
        role: 'ADMIN',
        emailVerified: false,
        accessToken: 'token',
        refreshToken: 'refresh',
        expiresAt: new Date().toISOString(),
        tokenType: 'Bearer',
      },
    })

    await useAuthStore.getState().login({
      email: 'fulltest@example.com',
      password: 'password',
    })

    const user = useAuthStore.getState().user
    expect(user).toMatchObject({
      id: 'test-id-456',
      email: 'fulltest@example.com',
      username: 'fulltest',
      displayName: 'Full Test User',
      avatarUrl: 'https://example.com/img.png',
      role: 'admin',
      emailVerified: false,
    })
  })
})
