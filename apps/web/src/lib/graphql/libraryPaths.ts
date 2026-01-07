/**
 * GraphQL operations for user library path management
 *
 * Contains queries and mutations for managing user-specific library paths:
 * - Query user's library paths
 * - Add new library path
 * - Remove library path
 * - Set primary library path
 * - Update library path label
 */

import { gql } from 'graphql-request'

/**
 * Get current user's library paths
 */
export const USER_LIBRARY_PATHS_QUERY = gql`
  query UserLibraryPaths {
    userLibraryPaths {
      id
      path
      label
      isPrimary
      createdAt
    }
  }
`

/**
 * Add a new library path for the user
 */
export const ADD_USER_LIBRARY_PATH_MUTATION = gql`
  mutation AddUserLibraryPath($path: String!, $label: String) {
    addUserLibraryPath(path: $path, label: $label) {
      id
      path
      label
      isPrimary
      createdAt
    }
  }
`

/**
 * Remove a library path
 */
export const REMOVE_USER_LIBRARY_PATH_MUTATION = gql`
  mutation RemoveUserLibraryPath($id: ID!) {
    removeUserLibraryPath(id: $id)
  }
`

/**
 * Set a library path as the user's primary
 */
export const SET_USER_PRIMARY_LIBRARY_MUTATION = gql`
  mutation SetUserPrimaryLibrary($id: ID!) {
    setUserPrimaryLibrary(id: $id) {
      id
      path
      label
      isPrimary
      createdAt
    }
  }
`

/**
 * Update a library path's label
 */
export const UPDATE_USER_LIBRARY_PATH_MUTATION = gql`
  mutation UpdateUserLibraryPath($id: ID!, $label: String!) {
    updateUserLibraryPath(id: $id, label: $label) {
      id
      path
      label
      isPrimary
      createdAt
    }
  }
`
