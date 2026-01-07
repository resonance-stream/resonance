/**
 * System settings types for the frontend admin pages
 */

import type { ServiceType, SystemSettingInfo, ConnectionTestResult } from '@resonance/shared-types'

// Re-export shared types
export type { ServiceType, SystemSettingInfo, ConnectionTestResult }

// ============================================================================
// GraphQL Response Types
// ============================================================================

export interface SystemSettingsResponse {
  systemSettings: SystemSettingInfo[]
}

export interface SystemSettingResponse {
  systemSetting: SystemSettingInfo | null
}

export interface UpdateSystemSettingResponse {
  updateSystemSetting: SystemSettingInfo
}

export interface TestServiceConnectionResponse {
  testServiceConnection: ConnectionTestResult
}

// ============================================================================
// Service Metadata
// ============================================================================

export interface ServiceMetadata {
  type: ServiceType
  name: string
  description: string
  icon: string
  category: 'ai' | 'integrations' | 'storage'
  configFields: ConfigField[]
  hasSecret: boolean
  secretLabel?: string
}

export interface ConfigField {
  key: string
  label: string
  type: 'text' | 'url' | 'number' | 'boolean' | 'select'
  placeholder?: string
  required?: boolean
  options?: { value: string; label: string }[]
}

// Service metadata definitions
export const SERVICE_METADATA: Record<ServiceType, ServiceMetadata> = {
  OLLAMA: {
    type: 'OLLAMA',
    name: 'Ollama',
    description: 'Local AI inference for recommendations and natural language search',
    icon: 'brain',
    category: 'ai',
    configFields: [
      { key: 'url', label: 'URL', type: 'url', placeholder: 'http://localhost:11434', required: true },
      { key: 'model', label: 'Chat Model', type: 'text', placeholder: 'mistral', required: false },
      { key: 'embedding_model', label: 'Embedding Model', type: 'text', placeholder: 'nomic-embed-text', required: false },
    ],
    hasSecret: false,
  },
  LIDARR: {
    type: 'LIDARR',
    name: 'Lidarr',
    description: 'Music collection manager for automatic library management',
    icon: 'music',
    category: 'integrations',
    configFields: [
      { key: 'url', label: 'URL', type: 'url', placeholder: 'http://localhost:8686', required: true },
    ],
    hasSecret: true,
    secretLabel: 'API Key',
  },
  LASTFM: {
    type: 'LASTFM',
    name: 'Last.fm',
    description: 'Scrobbling and music discovery integration',
    icon: 'radio',
    category: 'integrations',
    configFields: [],
    hasSecret: true,
    secretLabel: 'API Key',
  },
  MEILISEARCH: {
    type: 'MEILISEARCH',
    name: 'Meilisearch',
    description: 'Fast full-text search engine',
    icon: 'search',
    category: 'integrations',
    configFields: [
      { key: 'url', label: 'URL', type: 'url', placeholder: 'http://localhost:7700', required: true },
    ],
    hasSecret: true,
    secretLabel: 'Master Key',
  },
  MUSIC_LIBRARY: {
    type: 'MUSIC_LIBRARY',
    name: 'Music Library',
    description: 'Path to your music files',
    icon: 'folder',
    category: 'storage',
    configFields: [
      { key: 'path', label: 'Library Path', type: 'text', placeholder: '/path/to/music', required: true },
    ],
    hasSecret: false,
  },
}

// Group services by category
export const SERVICE_CATEGORIES = {
  ai: {
    label: 'AI Services',
    services: ['OLLAMA'] as ServiceType[],
  },
  integrations: {
    label: 'Integrations',
    services: ['LIDARR', 'LASTFM', 'MEILISEARCH'] as ServiceType[],
  },
  storage: {
    label: 'Storage',
    services: ['MUSIC_LIBRARY'] as ServiceType[],
  },
}
