<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { SxoButton, SxoInput } from '@sxo/vue'
import { 
    getRepositories, 
    updateRepository, 
    deleteRepository, 
    addCollaborator, 
    getCollaborators,
    removeCollaborator,
    createRepository,
    getOrganizations,
    type Repository as ApiRepository,
    type User,
    type Organization
} from '../api'

interface Repository extends ApiRepository {
    // Mapped fields
    stars: number
    lastUpdated: string
    
    // Mocked/UI fields
    language: string
    contentHash: string
    size: string
    team?: string
    status?: 'stable' | 'building' | 'failed'
    activity?: number[] // For sparkline
    tags?: string[]
    project?: string
    docsUrl?: string
    configUrl?: string
    contributors?: string[]
}

const searchQuery = ref('')
const filterType = ref('all')
const activeOrg = ref('all')
const activeLang = ref('all')
const viewMode = ref<'grid' | 'list'>('grid')

const languages = ['RUST', 'TYPESCRIPT', 'GO', 'MARKDOWN', 'CSS']

const globalStats = [
    { label: 'TOTAL REPOSITORIES', value: '42', icon: 'files', color: 'text-theme-primary' },
    { label: 'ACTIVE PATCHES', value: '1,284', icon: 'connection', color: 'text-theme-orange' },
    { label: 'BUILD SUCCESS', value: '98.2%', icon: 'check', color: 'text-theme-green' },
]

const recentActivity = [
    { id: 1, user: 'Felix', action: 'merged patch', repository: 'ys-core-engine', time: '2m ago', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Felix' },
    { id: 2, user: 'Aneka', action: 'pushed to', repository: 'ys-client-h5', time: '15m ago', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Aneka' },
]

const repositories = ref<Repository[]>([])
const organizationsList = ref<Organization[]>([])
const selectedRepository = ref<Repository | null>(null)
const showSettings = ref(false)
const showNewRepositoryModal = ref(false)
const collaborators = ref<User[]>([])
const newCollabUsername = ref('')
const newCollabPermission = ref('read')
const isSaving = ref(false)
const isCreating = ref(false)

const newRepositoryData = ref({
    name: '',
    description: '',
    visibility: 'public',
    primary_language: 'RUST',
    organization_id: ''
})

const openNewRepositoryModal = () => {
    newRepositoryData.value = {
        name: '',
        description: '',
        visibility: 'public',
        primary_language: 'RUST',
        organization_id: ''
    }
    showNewRepositoryModal.value = true
}

const handleCreateRepository = async () => {
    if (!newRepositoryData.value.name) return
    
    isCreating.value = true
    try {
        await createRepository(newRepositoryData.value)
        await loadRepositories()
        showNewRepositoryModal.value = false
    } catch (error) {
        console.error('Failed to create repository:', error)
        alert('Failed to create repository')
    } finally {
        isCreating.value = false
    }
}

const getRepositoryPath = (repository: Repository) => {
    // Find the organization name from the organizationsList
    const organization = repository.organization_id 
        ? organizationsList.value.find(o => o.id === repository.organization_id)?.name || 'default'
        : 'users' // Default to 'users' for personal repositories if not in an organization
    
    return { organization, name: repository.name }
}

const openSettings = async (repository: Repository) => {
    selectedRepository.value = JSON.parse(JSON.stringify(repository)) // Deep copy to avoid direct binding while editing
    showSettings.value = true
    
    // Load collaborators
    try {
        const { organization, name } = getRepositoryPath(repository)
        collaborators.value = await getCollaborators(organization, name)
    } catch (error) {
        console.error('Failed to load collaborators:', error)
        collaborators.value = []
    }
}

const saveSettings = async () => {
    if (!selectedRepository.value) return
    
    isSaving.value = true
    try {
        const { organization, name } = getRepositoryPath(selectedRepository.value)
        await updateRepository(organization, name, {
            visibility: selectedRepository.value.visibility,
            description: selectedRepository.value.description,
            primary_language: selectedRepository.value.primary_language,
            organization_id: selectedRepository.value.organization_id
        })
        
        // Refresh local list
        await loadRepositories()
        showSettings.value = false
    } catch (error) {
        console.error('Failed to save settings:', error)
        alert('Failed to save settings')
    } finally {
        isSaving.value = false
    }
}

const handleDeleteRepository = async () => {
    if (!selectedRepository.value) return
    if (!confirm(`Are you sure you want to delete ${selectedRepository.value.name}? This action cannot be undone.`)) return
    
    try {
        const { organization, name } = getRepositoryPath(selectedRepository.value)
        await deleteRepository(organization, name)
        await loadRepositories()
        showSettings.value = false
    } catch (error) {
        console.error('Failed to delete repository:', error)
        alert('Failed to delete repository')
    }
}

const handleAddCollaborator = async () => {
    if (!selectedRepository.value || !newCollabUsername.value) return
    
    try {
        const { organization, name } = getRepositoryPath(selectedRepository.value)
        await addCollaborator(organization, name, newCollabUsername.value, newCollabPermission.value)
        
        // Refresh collaborators
        collaborators.value = await getCollaborators(organization, name)
        newCollabUsername.value = ''
    } catch (error) {
        console.error('Failed to add collaborator:', error)
        alert('Failed to add collaborator: ' + (error as any).response?.data || 'Unknown error')
    }
}

const handleRemoveCollaborator = async (username: string) => {
    if (!selectedRepository.value) return
    
    try {
        const { organization, name } = getRepositoryPath(selectedRepository.value)
        await removeCollaborator(organization, name, username)
        
        // Refresh collaborators
        collaborators.value = await getCollaborators(organization, name)
    } catch (error) {
        console.error('Failed to remove collaborator:', error)
        alert('Failed to remove collaborator')
    }
}

const loadRepositories = async () => {
    try {
        const [repositoriesResponse, organizationsResponse] = await Promise.all([
            getRepositories(),
            getOrganizations()
        ])
        
        organizationsList.value = organizationsResponse
        repositories.value = repositoriesResponse.map(repository => {
            const organization = organizationsResponse.find(o => o.id === repository.organization_id)
            return {
                ...repository,
                // Map backend fields to frontend fields
                stars: repository.stars_count,
                lastUpdated: new Date(repository.updated_at).toLocaleDateString(),
                language: repository.primary_language || 'RUST',
                
                // Use organization name as team if available
                team: organization ? organization.name : 'Personal',
                status: repository.quality === 'A' ? 'stable' : (repository.quality === 'F' ? 'failed' : 'building'),
                
                // Mock missing fields for UI consistency
                contentHash: 'P7D2K1',
                size: '124 MB',
                activity: Array.from({ length: 12 }, () => Math.floor(Math.random() * 100)),
                tags: [repository.primary_language || 'RUST', repository.visibility.toUpperCase()],
                project: organization ? organization.name : 'PERSONAL',
                contributors: []
            }
        })
        // Update global stats
        globalStats[0].value = repositories.value.length.toString()
    } catch (error) {
        console.error('Failed to load repositories:', error)
    }
}

onMounted(() => {
    loadRepositories()
})

const filteredRepositories = computed(() => {
    return repositories.value.filter(repository => {
        const searchLower = searchQuery.value.toLowerCase()
        const matchesSearch = repository.name.toLowerCase().includes(searchLower) || 
            (repository.description || '').toLowerCase().includes(searchLower) ||
            (repository.tags || []).some(tag => tag.toLowerCase().includes(searchLower))
        
        const matchesType = filterType.value === 'all' || repository.visibility === filterType.value
        
        const matchesOrg = activeOrg.value === 'all' || (
            activeOrg.value === 'personal' ? !repository.organization_id : repository.organization_id === activeOrg.value
        )
        
        const matchesLang = activeLang.value === 'all' || repository.primary_language === activeLang.value 
        
        return matchesSearch && matchesType && matchesOrg && matchesLang
    })
})

const sidebarOrganizations = computed(() => [
    { id: 'all', name: '所有项目', icon: 'ALL' },
    ...organizationsList.value.map(organization => ({ id: organization.id, name: organization.name, icon: 'cloudy' })),
    { id: 'personal', name: '个人项目', icon: '👤' },
])

const quotas = [
    { label: 'STORAGE', value: 68, color: 'var(--theme-primary)' },
    { label: 'COMPUTE', value: 42, color: 'var(--theme-orange)' },
    { label: 'BANDWIDTH', value: 15, color: 'var(--theme-green)' },
]

const getLanguageColor = (lang: string) => {
    const colors: Record<string, string> = {
        'RUST': 'bg-theme-orange',
        'TYPESCRIPT': 'bg-theme-primary',
        'GO': 'bg-theme-green',
        'MARKDOWN': 'bg-theme-text-muted',
        'CSS': 'bg-theme-purple'
    }
    return colors[lang] || 'bg-theme-primary'
}

const getStatusColor = (status: string) => {
    switch (status) {
        case 'stable': return 'text-theme-green border-theme-green/30'
        case 'building': return 'text-theme-orange border-theme-orange/30 animate-pulse'
        case 'failed': return 'text-theme-red border-theme-red/30'
        default: return 'text-theme-text-muted border-theme-border'
    }
}

const generateSparklinePoints = (data: number[]) => {
    const width = 100
    const height = 30
    const max = Math.max(...data)
    const min = Math.min(...data)
    const range = max - min || 1
  
    return data.map((val, i) => {
        const x = (i / (data.length - 1)) * width
        const y = height - ((val - min) / range) * height
        return `${x},${y}`
    }).join(' ')
}
</script>

<template>
  <div class="flex flex-col min-h-screen bg-theme-bg p-6">
    <!-- Compact Header -->
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-4">
        <h1 class="text-xl font-black text-theme-text tracking-tight uppercase">Repositories</h1>
        <div class="h-4 w-px bg-theme-border opacity-50"></div>
        <div class="flex items-center gap-2">
          <div v-for="stat in globalStats" :key="stat.label" class="flex items-center gap-1.5 px-3 py-1 rounded border border-theme-border bg-theme-bg shadow-none">
            <span class="text-[9px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">{{ stat.label }}</span>
            <span class="text-xs font-black text-theme-text">{{ stat.value }}</span>
          </div>
        </div>
      </div>
      <div class="flex gap-2">
        <SxoButton variant="secondary" size="sm" class="!rounded px-4 !shadow-none hover:!border-theme-primary hover:!text-theme-primary">
          <div class="mr-1.5 opacity-70">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
          </div>
          IMPORT
        </SxoButton>
        <SxoButton variant="primary" size="sm" class="!rounded px-4 border-none !shadow-none hover:opacity-90" @click="openNewRepositoryModal">
          <div class="mr-1.5">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </div>
          NEW REPOSITORY
        </SxoButton>
      </div>
    </div>

    <div class="flex gap-6 flex-1">
      <!-- Compact Left Sidebar -->
      <div class="w-56 flex flex-col gap-4 shrink-0">
        <!-- Organizations Section -->
        <div class="bg-theme-bg rounded border border-theme-border p-3 shadow-none">
          <div class="flex items-center gap-2 mb-3 px-1">
            <span class="text-[9px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">Organizations</span>
          </div>
          <div class="space-y-1">
            <div
              v-for="organization in sidebarOrganizations" :key="organization.id" 
              :class="[
                'flex items-center gap-2.5 p-2 rounded cursor-pointer transition-all duration-200 border !shadow-none',
                activeOrg === organization.id ? 'border-theme-primary bg-theme-primary/5 text-theme-primary' : 'border-transparent hover:bg-theme-bg-subtle text-theme-text'
              ]"
              @click="activeOrg = organization.id">
              <div
                :class="[
                  'w-6 h-6 rounded flex items-center justify-center font-bold text-[10px] border !shadow-none',
                  activeOrg === organization.id ? 'border-theme-primary/20 bg-theme-primary/10' : 'border-theme-border bg-theme-bg-subtle text-theme-text-muted'
                ]">
                <svg v-if="organization.icon === 'cloudy'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.5 19c2.5 0 4.5-2 4.5-4.5 0-2.3-1.7-4.2-3.9-4.5-1.1-4.1-4.8-7-9.1-7-4.3 0-8.1 3-9.1 7C1.7 10.3 0 12.2 0 14.5 0 17 2 19 4.5 19H17.5z"/></svg>
                <span v-else>{{ organization.icon }}</span>
              </div>
              <span class="text-xs font-bold">{{ organization.name }}</span>
            </div>
          </div>
        </div>

        <!-- Compact Quotas -->
        <div class="bg-theme-bg rounded border border-theme-border p-3 shadow-none">
          <div class="flex items-center gap-2 mb-4 px-1">
            <span class="text-[9px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">Quotas</span>
          </div>
          <div class="space-y-4 px-1">
            <div v-for="quota in quotas" :key="quota.label" class="space-y-1.5">
              <div class="flex justify-between text-[8px] font-bold text-theme-text-muted tracking-widest">
                <span>{{ quota.label }}</span>
                <span class="text-theme-text opacity-80">{{ quota.value }}%</span>
              </div>
              <div class="h-1 bg-theme-bg-subtle rounded-full overflow-hidden">
                <div
                  class="h-full transition-all duration-1000" 
                  :style="{ width: `${quota.value}%`, backgroundColor: quota.color }"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Compact Activity Feed -->
        <div class="bg-theme-bg rounded border border-theme-border p-3 shadow-none">
          <div class="flex items-center gap-2 mb-3 px-1">
            <span class="text-[9px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">Recent Activity</span>
          </div>
          <div class="space-y-3 px-1">
            <div v-for="act in recentActivity" :key="act.id" class="flex gap-2">
              <img :src="act.avatar" class="w-5 h-5 rounded-full shrink-0" />
              <div class="flex flex-col min-w-0">
                <p class="text-[10px] text-theme-text leading-tight">
                  <span class="font-bold">{{ act.user }}</span> {{ act.action }} 
                  <span class="text-theme-primary font-medium truncate block">{{ act.repository }}</span>
                </p>
                <span class="text-[8px] text-theme-text-muted opacity-60">{{ act.time }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Main Content -->
      <div class="flex-1 flex flex-col gap-6 min-w-0">
        <!-- Controls Bar -->
        <div class="flex items-center justify-between gap-4">
          <div class="flex items-center gap-3 flex-1">
            <div class="relative group">
              <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-theme-text-muted">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
              </div>
              <SxoInput
                v-model="searchQuery"
                placeholder="SEARCH NAME, DESC, OR TAGS..."
                class="!w-72 !pl-9 !h-9 !rounded-xl !bg-theme-bg-subtle !border-theme-border focus-within:!border-theme-primary focus-within:!bg-theme-bg-lighter transition-all"
              />
            </div>
            
            <select v-model="filterType" class="w-28 h-9 px-3 bg-theme-bg-subtle border border-theme-border rounded-xl text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all">
              <option value="all">ALL VIS</option>
              <option value="public">PUBLIC</option>
              <option value="private">PRIVATE</option>
              <option value="internal">INTERNAL</option>
            </select>

            <select v-model="activeLang" class="w-28 h-9 px-3 bg-theme-bg-subtle border border-theme-border rounded-xl text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all">
              <option value="all">ALL LANG</option>
              <option v-for="lang in languages" :key="lang" :value="lang">{{ lang }}</option>
            </select>

            <div class="flex items-center bg-theme-bg rounded border border-theme-border p-0.5 shadow-none">
              <button
                :class="['p-1.5 rounded transition-all shadow-none', viewMode === 'grid' ? 'bg-theme-bg-subtle text-theme-primary border border-theme-border/50' : 'text-theme-text-muted hover:text-theme-text border border-transparent']" 
                @click="viewMode = 'grid'">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>
              </button>
              <button
                :class="['p-1.5 rounded transition-all shadow-none', viewMode === 'list' ? 'bg-theme-bg-subtle text-theme-primary border border-theme-border/50' : 'text-theme-text-muted hover:text-theme-text border border-transparent']" 
                @click="viewMode = 'list'">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>
              </button>
            </div>
          </div>
          <div class="text-[10px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">
            {{ filteredRepositories.length }} REPOSITORIES FOUND
          </div>
        </div>

        <!-- Repository Content -->
        <div v-if="viewMode === 'grid'" class="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-4">
          <div
            v-for="repository in filteredRepositories" :key="repository.id" 
            class="bg-theme-bg rounded border border-theme-border p-4 hover:border-theme-primary transition-all duration-200 group relative overflow-hidden flex flex-col shadow-none">
            
            <div class="flex justify-between items-start mb-2">
              <div class="flex flex-col min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <span v-if="repository.project" class="text-[9px] font-bold text-theme-primary uppercase tracking-widest border border-theme-primary/20 px-1.5 py-0.5 rounded-sm">{{ repository.project }}</span>
                  <span v-if="repository.project" class="text-theme-border opacity-50">/</span>
                  <h3 class="text-sm font-black text-theme-text group-hover:text-theme-primary transition-colors cursor-pointer truncate tracking-tight">{{ repository.name }}</h3>
                </div>
                <div class="flex items-center gap-2">
                  <span :class="['text-[8px] font-bold px-1.5 py-0.5 rounded-sm border uppercase tracking-tight', getStatusColor(repository.visibility === 'public' ? 'stable' : 'default')]">
                    {{ repository.visibility }}
                  </span>
                  <div :class="['text-[8px] font-bold px-2 py-0.5 rounded-sm border uppercase tracking-widest', getStatusColor(repository.status)]">
                    {{ repository.status }}
                  </div>
                </div>
              </div>
              <div class="flex items-center gap-1">
                <SxoButton v-if="repository.docsUrl" variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded-full !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !shadow-none">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>
                </SxoButton>
                <SxoButton v-if="repository.configUrl" variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded-full !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !shadow-none">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1.1-1.51V3a2 2 0 0 1-2-2 2 2 0 0 1-2 2v.09a1.65 1.65 0 0 0-1 1.51 1.65 1.65 0 0 0-1.82-.33l-.06-.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0 2.83l.06.06a1.65 1.65 0 0 0 .33 1.82V9a1.65 1.65 0 0 0-1.51 1H3a2 2 0 0 1-2 2 2 2 0 0 1 2 2h.09a1.65 1.65 0 0 0 1.51 1 1.65 1.65 0 0 0-.33 1.82l-.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1 2.83 0l.06-.06a1.65 1.65 0 0 0 1.82.33H15a1.65 1.65 0 0 0 1-1.51V21a2 2 0 0 1 2-2 2 2 0 0 1 2-2v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0 1.82.33l.06.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0-2.83l-.06-.06a1.65 1.65 0 0 0-.33-1.82V15a1.65 1.65 0 0 0 1.51-1H21a2 2 0 0 1 2-2 2 2 0 0 1-2-2h-.09a1.65 1.65 0 0 0-1.51-1z"/></svg>
                </SxoButton>
                <div class="relative group/menu">
                  <SxoButton variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded-full !bg-transparent !border-none !text-theme-text-muted hover:!text-theme-primary !shadow-none">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/></svg>
                  </SxoButton>
                  <div class="absolute right-0 top-full mt-1 w-32 bg-theme-bg border border-theme-border rounded-lg shadow-xl py-1 z-50 hidden group-hover/menu:block">
                    <button class="w-full text-left px-3 py-1.5 text-xs hover:bg-theme-bg-subtle transition-colors">仓库设置</button>
                    <button class="w-full text-left px-3 py-1.5 text-xs hover:bg-theme-bg-subtle transition-colors">网络图谱</button>
                    <button class="w-full text-left px-3 py-1.5 text-xs hover:bg-theme-bg-subtle text-red-500 transition-colors border-t border-theme-border mt-1">危险区域</button>
                  </div>
                </div>
              </div>
            </div>

            <p class="text-[11px] text-theme-text-muted leading-relaxed mb-3 line-clamp-2 h-8 font-medium">
              {{ repository.description }}
            </p>

            <!-- Stats & Meta -->
            <div class="grid grid-cols-3 gap-2 mb-4">
              <div class="bg-theme-bg-subtle/50 rounded p-2 border border-theme-border/50">
                <span class="text-[8px] font-bold text-theme-text-muted uppercase tracking-widest block mb-0.5">Patches</span>
                <span class="text-xs font-black text-theme-text">{{ repository.patch_count }}</span>
              </div>
              <div class="bg-theme-bg-subtle/50 rounded p-2 border border-theme-border/50">
                <span class="text-[8px] font-bold text-theme-text-muted uppercase tracking-widest block mb-0.5">Merged</span>
                <span class="text-xs font-black text-theme-text">{{ repository.merged_patches }}</span>
              </div>
              <div class="bg-theme-bg-subtle/50 rounded p-2 border border-theme-border/50">
                <span class="text-[8px] font-bold text-theme-text-muted uppercase tracking-widest block mb-0.5">Stars</span>
                <span class="text-xs font-black text-theme-text">{{ repository.stars }}</span>
              </div>
            </div>

            <!-- Tags & Lang -->
            <div class="flex items-center justify-between mb-4">
              <div class="flex flex-wrap gap-1">
                <span
                  v-for="tag in repository.tags" :key="tag" 
                  class="text-[8px] font-bold px-1.5 py-0.5 rounded-sm border border-theme-border text-theme-text-muted hover:border-theme-primary hover:text-theme-primary cursor-pointer transition-colors">
                  #{{ tag }}
                </span>
              </div>
              <div class="flex items-center gap-1 text-[9px] font-bold text-theme-text-muted uppercase tracking-widest">
                <span :class="['w-1.5 h-1.5 rounded-full', getLanguageColor(repository.language)]"></span>
                <span>{{ repository.language }}</span>
              </div>
            </div>

            <!-- Quick Actions & Activity -->
            <div class="mt-auto pt-3 border-t border-theme-border/30 flex items-center justify-between">
              <div class="flex gap-1">
                <SxoButton variant="secondary" size="sm" class="!text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !text-[10px] !px-2 h-7 !rounded-sm !shadow-none">
                  <div class="mr-1 opacity-70">
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
                  </div>
                  SYNC
                </SxoButton>
                <SxoButton variant="secondary" size="sm" class="!text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !text-[10px] !px-2 h-7 !rounded-sm !shadow-none">
                  <div class="mr-1 opacity-70">
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
                  </div>
                  IDE
                </SxoButton>
              </div>
              <div class="flex flex-col items-end">
                <svg width="50" height="15" class="opacity-40 group-hover:opacity-100 transition-opacity">
                  <polyline
                    fill="none"
                    stroke="var(--theme-primary)"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    :points="generateSparklinePoints(repository.activity)"
                  />
                </svg>
              </div>
            </div>
          </div>
        </div>

        <!-- List View (Ultra Dense) -->
        <div v-else class="bg-theme-bg-lighter rounded-lg border border-theme-border overflow-hidden">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="bg-theme-bg-subtle/30 text-[9px] font-bold text-theme-text-muted uppercase tracking-widest border-b border-theme-border">
                <th class="px-6 py-3 w-8"></th>
                <th class="px-4 py-3">Repository</th>
                <th class="px-4 py-3">Project</th>
                <th class="px-4 py-3">Tags</th>
                <th class="px-4 py-3">Status</th>
                <th class="px-4 py-3">Links</th>
                <th class="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-theme-border/50">
              <tr
                v-for="repository in filteredRepositories" :key="repository.id" 
                class="hover:bg-theme-bg-subtle/20 transition-colors group">
                <td class="px-6 py-3">
                  <div :class="['w-1.5 h-1.5 rounded-full', getLanguageColor(repository.language)]"></div>
                </td>
                <td class="px-4 py-3">
                  <div class="flex flex-col min-w-[200px]">
                    <div class="flex items-center gap-2">
                      <span class="text-sm font-bold text-theme-text group-hover:text-theme-primary transition-colors cursor-pointer">{{ repository.name }}</span>
                      <span class="text-[8px] font-bold px-1 py-0.5 rounded border border-theme-border text-theme-text-muted uppercase">{{ repository.visibility }}</span>
                    </div>
                    <span class="text-[10px] text-theme-text-muted line-clamp-1 opacity-60">{{ repository.description }}</span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  <span class="text-[10px] font-bold text-theme-primary uppercase tracking-widest opacity-80">{{ repository.project || '-' }}</span>
                </td>
                <td class="px-4 py-3">
                  <div class="flex gap-1">
                    <span v-for="tag in repository.tags.slice(0, 2)" :key="tag" class="text-[8px] text-theme-text-muted opacity-60">#{{ tag }}</span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div :class="['text-[8px] font-bold px-2 py-0.5 rounded-full inline-block uppercase tracking-widest', getStatusColor(repository.status)]">
                    {{ repository.status }}
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div class="flex gap-1">
                    <SxoButton v-if="repository.docsUrl" variant="ghost" size="sm" class="!w-6 !h-6 !p-0 !rounded-full !bg-transparent !border-none !text-theme-text-muted hover:!text-theme-primary !shadow-none">
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>
                    </SxoButton>
                    <SxoButton v-if="repository.configUrl" variant="ghost" size="sm" class="!w-6 !h-6 !p-0 !rounded-full !bg-transparent !border-none !text-theme-text-muted hover:!text-theme-primary !shadow-none">
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1.1-1.51V3a2 2 0 0 1-2-2 2 2 0 0 1-2 2v.09a1.65 1.65 0 0 0-1 1.51 1.65 1.65 0 0 0-1.82-.33l-.06-.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0 2.83l.06.06a1.65 1.65 0 0 0 .33 1.82V9a1.65 1.65 0 0 0-1.51 1H3a2 2 0 0 1-2 2 2 2 0 0 1 2 2h.09a1.65 1.65 0 0 0 1.51 1 1.65 1.65 0 0 0-.33 1.82l-.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1 2.83 0l.06-.06a1.65 1.65 0 0 0 1.82.33H15a1.65 1.65 0 0 0 1-1.51V21a2 2 0 0 1 2-2 2 2 0 0 1 2-2v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0 1.82.33l.06.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0-2.83l-.06-.06a1.65 1.65 0 0 0-.33-1.82V15a1.65 1.65 0 0 0 1.51-1H21a2 2 0 0 1 2-2 2 2 0 0 1-2-2h-.09a1.65 1.65 0 0 0-1.51-1z"/></svg>
                    </SxoButton>
                  </div>
                </td>
                <td class="px-4 py-3 text-right">
                  <div class="flex justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <SxoButton variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !shadow-none" @click.stop="openSettings(repository)">
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
                    </SxoButton>
                    <SxoButton variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !shadow-none">
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                    </SxoButton>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Repository Settings Modal -->
    <div v-if="showSettings && selectedRepository" class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-theme-bg/80 backdrop-blur-sm">
      <div class="bg-theme-bg border border-theme-border rounded-xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh]">
        <!-- Modal Header -->
        <div class="px-6 py-4 border-b border-theme-border flex items-center justify-between">
          <div class="flex flex-col">
            <h2 class="text-lg font-black text-theme-text uppercase tracking-tight">Repository Settings</h2>
            <p class="text-xs text-theme-text-muted font-bold opacity-60 uppercase">{{ selectedRepository.name }}</p>
          </div>
          <button @click="showSettings = false" class="text-theme-text-muted hover:text-theme-primary transition-colors">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>

        <!-- Modal Body -->
        <div class="flex-1 overflow-y-auto p-6 space-y-8">
          <!-- Basic Info Section -->
          <section class="space-y-4">
            <h3 class="text-[10px] font-black text-theme-text-muted uppercase tracking-[0.2em]">Basic Information</h3>
            <div class="space-y-4">
              <div class="space-y-1.5">
                <label class="text-[10px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Repository Description</label>
                <SxoInput 
                  v-model="selectedRepository.description"
                  placeholder="ADD A SHORT DESCRIPTION..." 
                  class="!w-full !h-10 !rounded-lg !bg-theme-bg-subtle !border-theme-border" 
                />
              </div>
              <div class="space-y-1.5">
                <label class="text-[10px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Primary Language</label>
                <select 
                  v-model="selectedRepository.primary_language"
                  class="w-full h-10 px-3 bg-theme-bg-subtle border border-theme-border rounded-lg text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
                >
                  <option v-for="lang in languages" :key="lang" :value="lang">{{ lang }}</option>
                </select>
              </div>
            </div>
          </section>

          <!-- Visibility Section -->
          <section class="space-y-3">
            <h3 class="text-[10px] font-black text-theme-text-muted uppercase tracking-[0.2em]">Visibility & Access</h3>
          <div class="space-y-1.5">
            <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Organization (Optional)</label>
            <select 
              v-model="selectedRepository.organization_id"
              class="w-full h-10 px-3 bg-theme-bg-subtle border border-theme-border rounded-lg text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
            >
              <option value="">Personal Project</option>
              <option v-for="org in organizationsList" :key="org.id" :value="org.id">{{ org.name }}</option>
            </select>
          </div>

          <div class="grid grid-cols-2 gap-4">
              <div 
                @click="selectedRepository.visibility = 'public'"
                :class="['p-4 rounded-xl border-2 cursor-pointer transition-all', selectedRepository.visibility === 'public' ? 'border-theme-primary bg-theme-primary/5' : 'border-theme-border hover:border-theme-primary/30']">
                <div class="flex items-center gap-3 mb-2">
                  <div class="w-8 h-8 rounded bg-theme-green/10 flex items-center justify-center text-theme-green">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
                  </div>
                  <span class="font-bold text-sm uppercase tracking-tight">Public</span>
                </div>
                <p class="text-[10px] text-theme-text-muted leading-relaxed">Anyone can see this repository. You choose who can commit.</p>
              </div>
              <div 
                @click="selectedRepository.visibility = 'private'"
                :class="['p-4 rounded-xl border-2 cursor-pointer transition-all', selectedRepository.visibility === 'private' ? 'border-theme-primary bg-theme-primary/5' : 'border-theme-border hover:border-theme-primary/30']">
                <div class="flex items-center gap-3 mb-2">
                  <div class="w-8 h-8 rounded bg-theme-orange/10 flex items-center justify-center text-theme-orange">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
                  </div>
                  <span class="font-bold text-sm uppercase tracking-tight">Private</span>
                </div>
                <p class="text-[10px] text-theme-text-muted leading-relaxed">Only you and collaborators can see this repository.</p>
              </div>
            </div>
          </section>

          <!-- Collaborators Section -->
          <section class="space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-[10px] font-black text-theme-text-muted uppercase tracking-[0.2em]">Collaborators</h3>
            </div>
            
            <div class="space-y-2">
              <div class="flex gap-2">
                <SxoInput 
                  v-model="newCollabUsername"
                  placeholder="ENTER USERNAME..." 
                  class="flex-1 !h-10 !rounded-lg !bg-theme-bg-subtle !border-theme-border" 
                  @keyup.enter="handleAddCollaborator"
                />
                <select 
                  v-model="newCollabPermission"
                  class="w-24 h-10 px-2 bg-theme-bg-subtle border border-theme-border rounded-lg text-[10px] font-black text-theme-text uppercase focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
                >
                  <option value="read">READ</option>
                  <option value="write">WRITE</option>
                  <option value="admin">ADMIN</option>
                </select>
                <SxoButton 
                  variant="primary" 
                  size="sm" 
                  class="!h-10 !px-6 !rounded-lg !shadow-none uppercase font-black text-[10px] tracking-widest"
                  @click="handleAddCollaborator"
                >
                  ADD
                </SxoButton>
              </div>
              
              <div class="bg-theme-bg-subtle/30 rounded-lg border border-theme-border divide-y divide-theme-border/50 max-h-40 overflow-y-auto">
                <div v-if="collaborators.length === 0" class="p-4 text-center text-[10px] text-theme-text-muted uppercase font-bold opacity-50">
                  No collaborators added yet
                </div>
                <div v-for="user in collaborators" :key="user.id" class="p-3 flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <img v-if="user.avatar_url" :src="user.avatar_url" class="w-8 h-8 rounded-full" />
                    <div v-else class="w-8 h-8 rounded-full bg-theme-primary/20 flex items-center justify-center text-theme-primary font-bold text-xs uppercase">
                      {{ user.username[0] }}
                    </div>
                    <div class="flex flex-col">
                      <span class="text-xs font-bold text-theme-text uppercase">{{ user.username }}</span>
                      <span class="text-[9px] text-theme-text-muted font-bold opacity-60 uppercase">Collaborator</span>
                    </div>
                  </div>
                  <SxoButton 
                    variant="ghost" 
                    size="sm" 
                    class="!w-8 !h-8 !p-0 !rounded-lg hover:!bg-red-500/10 hover:!text-red-500 !text-theme-text-muted transition-colors"
                    @click="handleRemoveCollaborator(user.username)"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                  </SxoButton>
                </div>
              </div>
            </div>
          </section>

          <!-- Danger Zone -->
          <section class="space-y-3 pt-4 border-t border-theme-border">
            <h3 class="text-[10px] font-black text-red-500 uppercase tracking-[0.2em]">Danger Zone</h3>
            <div class="p-4 rounded-xl border-2 border-red-500/20 bg-red-500/5 flex items-center justify-between">
              <div class="flex flex-col">
                <span class="text-sm font-bold text-theme-text uppercase tracking-tight">Delete Repository</span>
                <span class="text-[10px] text-theme-text-muted leading-relaxed">Once you delete a repository, there is no going back.</span>
              </div>
              <SxoButton 
                variant="secondary" 
                size="sm" 
                class="!text-red-500 !border-red-500/30 hover:!bg-red-500 hover:!text-white !font-black !text-[10px] !tracking-widest !rounded-lg uppercase"
                @click="handleDeleteRepository"
              >
                DELETE
              </SxoButton>
            </div>
          </section>
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-4 border-t border-theme-border bg-theme-bg-subtle/30 flex justify-end gap-3">
          <SxoButton variant="secondary" @click="showSettings = false" class="!font-black !text-[10px] !tracking-widest !rounded-lg uppercase">CANCEL</SxoButton>
          <SxoButton 
            variant="primary" 
            @click="saveSettings" 
            :loading="isSaving"
            class="!font-black !text-[10px] !tracking-widest !rounded-lg uppercase !shadow-none"
          >
            SAVE CHANGES
          </SxoButton>
        </div>
      </div>
    </div>

    <!-- New Repository Modal -->
    <div v-if="showNewRepositoryModal" class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-theme-bg/80 backdrop-blur-sm">
      <div class="bg-theme-bg border border-theme-border rounded-xl shadow-2xl w-full max-w-xl overflow-hidden flex flex-col max-h-[90vh]">
        <!-- Modal Header -->
        <div class="px-6 py-4 border-b border-theme-border flex items-center justify-between">
          <div class="flex flex-col">
            <h2 class="text-lg font-black text-theme-text uppercase tracking-tight">Create New Repository</h2>
            <p class="text-xs text-theme-text-muted font-bold opacity-60 uppercase">Start a new project</p>
          </div>
          <button @click="showNewRepositoryModal = false" class="text-theme-text-muted hover:text-theme-primary transition-colors">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>

        <!-- Modal Body -->
        <div class="flex-1 overflow-y-auto p-6 space-y-6">
          <div class="space-y-1.5">
            <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Repository Name</label>
            <SxoInput 
              v-model="newRepositoryData.name"
              placeholder="E.G. MY-AWESOME-PROJECT" 
              class="!w-full !h-10 !rounded-lg !bg-theme-bg-subtle !border-theme-border focus-within:!border-theme-primary transition-all" 
            />
          </div>

          <div class="space-y-1.5">
            <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Description (Optional)</label>
            <SxoInput 
              v-model="newRepositoryData.description"
              placeholder="WHAT IS THIS PROJECT ABOUT?" 
              class="!w-full !h-10 !rounded-lg !bg-theme-bg-subtle !border-theme-border focus-within:!border-theme-primary transition-all" 
            />
          </div>

          <div class="space-y-1.5">
            <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Organization (Optional)</label>
            <select 
              v-model="newRepositoryData.organization_id"
              class="w-full h-10 px-3 bg-theme-bg-subtle border border-theme-border rounded-lg text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
            >
              <option value="">Personal Project</option>
              <option v-for="org in organizationsList" :key="org.id" :value="org.id">{{ org.name }}</option>
            </select>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-1.5">
              <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Primary Language</label>
              <select 
                v-model="newRepositoryData.primary_language"
                class="w-full h-10 px-3 bg-theme-bg-subtle border border-theme-border rounded-lg text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
              >
                <option v-for="lang in languages" :key="lang" :value="lang">{{ lang }}</option>
              </select>
            </div>
            <div class="space-y-1.5">
              <label class="text-[10px] font-black text-theme-text-muted uppercase tracking-widest opacity-60 px-1">Visibility</label>
              <select 
                v-model="newRepositoryData.visibility"
                class="w-full h-10 px-3 bg-theme-bg-subtle border border-theme-border rounded-lg text-xs text-theme-text focus:outline-none focus:ring-1 focus:ring-theme-primary transition-all"
              >
                <option value="public">PUBLIC</option>
                <option value="private">PRIVATE</option>
              </select>
            </div>
          </div>
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-4 border-t border-theme-border bg-theme-bg-subtle/30 flex justify-end gap-3">
          <SxoButton variant="secondary" @click="showNewRepositoryModal = false" class="!font-black !text-[10px] !tracking-widest !rounded-lg uppercase">CANCEL</SxoButton>
          <SxoButton 
            variant="primary" 
            @click="handleCreateRepository" 
            :loading="isCreating"
            :disabled="!newRepositoryData.name"
            class="!font-black !text-[10px] !tracking-widest !rounded-lg uppercase !shadow-none"
          >
            CREATE REPOSITORY
          </SxoButton>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 移除 Element Plus 相关样式，由 Tailwind 和 SXO 组件处理 */

/* Scrollbar styling for the whole page */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--theme-border);
  border-radius: 10px;
}
::-webkit-scrollbar-thumb:hover {
  background: var(--theme-primary);
}
</style>

<route lang="yaml">
meta:
  name: 仓库管理
</route>
