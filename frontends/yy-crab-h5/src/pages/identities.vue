<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { SxoButton } from '@sxo/vue'
import { getUsers, type User } from '../api'

const users = ref<User[]>([])
const loading = ref(true)

const loadUsers = async () => {
    loading.value = true
    try {
        users.value = await getUsers()
    } catch (error) {
        console.error('Failed to load users:', error)
    } finally {
        loading.value = false
    }
}

onMounted(() => {
    loadUsers()
})
</script>

<template>
  <div class="flex flex-col min-h-screen bg-theme-bg p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-4">
        <h1 class="text-xl font-black text-theme-text tracking-tight uppercase">Identities</h1>
        <div class="h-4 w-px bg-theme-border opacity-50"></div>
        <div class="flex items-center gap-2">
          <div class="flex items-center gap-1.5 px-3 py-1 rounded border border-theme-border bg-theme-bg">
            <span class="text-[9px] font-bold text-theme-text-muted uppercase tracking-widest opacity-60">Total Users</span>
            <span class="text-xs font-black text-theme-text">{{ users.length }}</span>
          </div>
        </div>
      </div>
      <div class="flex gap-2">
        <SxoButton variant="primary" size="sm" class="!rounded px-4 border-none !shadow-none hover:opacity-90">
          <div class="mr-1.5">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </div>
          CREATE USER
        </SxoButton>
      </div>
    </div>

    <!-- User Grid -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="animate-pulse text-theme-text-muted uppercase tracking-widest font-bold text-xs">Loading identities...</div>
    </div>
    <div v-else-if="users.length === 0" class="flex-1 flex flex-col items-center justify-center text-theme-text-muted">
      <div class="mb-6 opacity-20">
        <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
          <circle cx="12" cy="7" r="4"/>
        </svg>
      </div>
      <p class="text-sm uppercase tracking-widest font-bold">No users found</p>
    </div>
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      <div v-for="user in users" :key="user.id" class="bg-theme-bg rounded border border-theme-border p-4 hover:border-theme-primary transition-all duration-200 group relative overflow-hidden flex flex-col shadow-none">
        <div class="flex items-center gap-4">
          <div class="w-12 h-12 rounded-full overflow-hidden border border-theme-border bg-theme-bg-subtle shrink-0">
            <img v-if="user.avatar_url" :src="user.avatar_url" class="w-full h-full object-cover" />
            <div v-else class="w-full h-full flex items-center justify-center text-theme-text-muted font-bold text-xl uppercase">
              {{ user.username.charAt(0) }}
            </div>
          </div>
          <div class="flex flex-col min-w-0">
            <h3 class="text-sm font-black text-theme-text group-hover:text-theme-primary transition-colors truncate tracking-tight">{{ user.display_name }}</h3>
            <p class="text-[10px] text-theme-text-muted font-medium truncate opacity-60">@{{ user.username }}</p>
          </div>
        </div>
        
        <div class="mt-4 pt-3 border-t border-theme-border/30 flex items-center justify-between">
          <div class="text-[8px] font-bold text-theme-text-muted uppercase tracking-widest">
            Joined {{ new Date(user.created_at).toLocaleDateString() }}
          </div>
          <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <SxoButton variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded-full !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-primary hover:!border-theme-primary !shadow-none">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
            </SxoButton>
            <SxoButton variant="ghost" size="sm" class="!w-7 !h-7 !p-0 !rounded-full !bg-theme-bg !border-theme-border !text-theme-text-muted hover:!text-theme-red hover:!border-theme-red !shadow-none">
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
            </SxoButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Scrollbar styling */
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
  name: 身份管理
</route>
