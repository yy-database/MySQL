// Mock API client implementation since @doki-land/git-crab-sdk is not available
interface ApiClient {
  get: (url: string) => Promise<any>
  post: (url: string, data?: any) => Promise<any>
  put: (url: string, data?: any) => Promise<any>
  delete: (url: string) => Promise<any>
  interceptors: {
    response: {
      use: (
        onFulfilled: (response: any) => any,
        onRejected: (error: any) => any
      ) => void
    }
  }
}

const api: ApiClient = {
  get: async (url: string) => Promise.resolve({ data: [] }),
  post: async (url: string, data?: any) => Promise.resolve({ data }),
  put: async (url: string, data?: any) => Promise.resolve({ data }),
  delete: async (url: string) => Promise.resolve({ data: {} }),
  interceptors: {
    response: {
      use: (onFulfilled: (response: any) => any, onRejected: (error: any) => any) => {}
    }
  }
}

// Response interceptor for admin-specific redirect
api.interceptors.response.use(
    (response) => response,
    (error) => {
        if (error.response && error.response.status === 401) {
            window.location.href = '/login'
        }
        return Promise.reject(error)
    }
)

export default api

// Export shared types
export interface Repository {
  id: string
  name: string
  description?: string
  visibility: string
  primary_language?: string
  organization_id?: string
  stars: number
  lastUpdated: string
  language: string
  contentHash: string
  size: string
  team?: string
  status?: 'stable' | 'building' | 'failed'
  activity?: number[]
  patch_count: number
  merged_patches: number
  tags?: string[]
}

export interface User {
  id: string
  username: string
  name: string
  avatar?: string
}

export interface Organization {
  id: string
  name: string
  description?: string
  members: number
}

// Export shared methods
export const getRepositories = async (): Promise<Repository[]> => {
  return [
    { id: '1', name: 'ys-platform/core', description: 'YS Platform Core', visibility: 'public', stars: 124, lastUpdated: '2h ago', language: 'TypeScript', contentHash: 'abc123', size: '1.2GB', patch_count: 156, merged_patches: 120, tags: ['core', 'platform'] },
    { id: '2', name: 'theme-ui/components', description: 'Theme UI Components', visibility: 'public', stars: 89, lastUpdated: '5h ago', language: 'Vue', contentHash: 'def456', size: '450MB', patch_count: 89, merged_patches: 75, tags: ['ui', 'components'] },
    { id: '3', name: 'impact/engine', description: 'Impact Engine', visibility: 'private', stars: 256, lastUpdated: '1d ago', language: 'Rust', contentHash: 'ghi789', size: '2.5GB', patch_count: 345, merged_patches: 290, tags: ['engine', 'performance'] }
  ]
}

export const getUsers = async (): Promise<User[]> => {
  return [
    { id: '1', username: 'alice', name: 'Alice', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Alice' },
    { id: '2', username: 'bob', name: 'Bob', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Bob' }
  ]
}

export const getOrganizations = async (): Promise<Organization[]> => {
  return [
    { id: '1', name: 'YS Platform', description: 'YS Platform Organization', members: 124 },
    { id: '2', name: 'OpenSource Foundation', description: 'OpenSource Foundation', members: 1250 }
  ]
}

export const starRepository = async (repoId: string): Promise<void> => {
  console.log('Starring repository:', repoId)
}

export const addCollaborator = async (repoId: string, userId: string): Promise<void> => {
  console.log('Adding collaborator:', userId, 'to repository:', repoId)
}

export const getCollaborators = async (repoId: string): Promise<User[]> => {
  return []
}

export const removeCollaborator = async (repoId: string, userId: string): Promise<void> => {
  console.log('Removing collaborator:', userId, 'from repository:', repoId)
}

export const updateRepository = async (repoId: string, data: any): Promise<void> => {
  console.log('Updating repository:', repoId, 'with data:', data)
}

export const deleteRepository = async (repoId: string): Promise<void> => {
  console.log('Deleting repository:', repoId)
}

export const createRepository = async (data: any): Promise<Repository> => {
  return {
    id: 'new-repo',
    name: data.name || 'new-repo',
    description: data.description,
    visibility: data.visibility || 'public',
    primary_language: data.primary_language,
    organization_id: data.organization_id,
    stars: 0,
    lastUpdated: 'just now',
    language: data.primary_language || 'Unknown',
    contentHash: 'new-hash',
    size: '0KB',
    patch_count: 0,
    merged_patches: 0
  }
}
