<script setup lang="ts">
import { useRouter } from 'vue-router'
import { SxoButton, SxoAvatar } from '@sxo/vue'

const router = useRouter()

const handleLogout = () => {
    sessionStorage.removeItem('ys-admin-token')
    router.push('/login')
}

const menuGroups = [
    {
        title: '系统核心',
        items: [
            { name: '控制台', icon: 'monitor', path: '/' },
            { name: '效能洞察', icon: 'analytics', path: '/analytics' },
        ]
    },
    {
        title: '身份与组织',
        items: [
            { name: '组织架构', icon: 'organization', path: '/organizations' },
            { name: '身份管理', icon: 'user', path: '/identities' },
            { name: '权限策略 (ABAC)', icon: 'key', path: '/permissions' },
        ]
    },
    {
        title: '补丁空间',
        items: [
            { name: '仓库管理', icon: 'repository', path: '/repositories' },
            { name: '补丁管理 (PDG)', icon: 'patch', path: '/patches' },
            { name: '代码审查', icon: 'review', path: '/reviews' },
        ]
    },
    {
        title: '项目管理',
        items: [
            { name: '智能问题 (Issue)', icon: 'issue', path: '/issues' },
            { name: '里程碑', icon: 'milestone', path: '/milestones' },
        ]
    },
    {
        title: '自动化与质量',
        items: [
            { name: '流水线 (CI/CD)', icon: 'pipeline', path: '/pipelines' },
            { name: '质量门禁', icon: 'quality', path: '/quality' },
        ]
    },
    {
        title: '知识与审计',
        items: [
            { name: '知识图谱', icon: 'knowledge', path: '/knowledge' },
            { name: '合规审计', icon: 'audit', path: '/audit' },
            { name: '系统设置', icon: 'settings', path: '/settings' },
        ]
    }
]
</script>

<template>
  <div class="h-screen bg-theme-bg flex overflow-hidden">
    <aside class="w-60 bg-theme-bg-lighter border-r border-theme-border flex flex-col shadow-sm z-20">
      <div class="p-4 flex items-center gap-2 border-b border-theme-border/50">
        <div class="w-8 h-8 bg-theme-primary rounded-lg flex items-center justify-center shadow-lg shadow-theme-primary/20">
          <span class="text-white font-bold text-sm">YS</span>
        </div>
        <div>
          <h2 class="text-sm font-bold text-theme-text leading-tight text-nowrap">补丁宇宙</h2>
          <p class="text-[9px] text-theme-text-muted uppercase tracking-widest font-bold opacity-70">Admin Console</p>
        </div>
      </div>
      
      <div class="flex-1 overflow-y-auto custom-scrollbar p-2">
        <nav class="space-y-4">
          <div v-for="group in menuGroups" :key="group.title">
            <div class="px-3 mb-1.5 text-[10px] font-bold text-theme-text-muted uppercase tracking-wider opacity-50">
              {{ group.title }}
            </div>
            <div class="space-y-0.5">
              <router-link 
                v-for="item in group.items" 
                :key="item.path" 
                :to="item.path"
                class="flex items-center px-3 h-9 rounded-lg transition-colors group no-underline"
                :class="[$route.path === item.path ? 'bg-theme-primary/10 text-theme-primary' : 'text-theme-text-muted hover:bg-theme-bg-subtle']"
              >
                <div class="mr-2 opacity-70 group-hover:opacity-100 transition-opacity">
                  <svg v-if="item.icon === 'monitor'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M28 22h-4v-2h4v-12H4v12h4v2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h24a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2zM18 24h-4v-2h4zM22 28H10a2 2 0 0 1-2-2v-2h2v2h12v-2h2v2a2 2 0 0 1-2 2z"/></svg>
                  <svg v-else-if="item.icon === 'analytics'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 4H6a2 2 0 0 0-2 2v20a2 2 0 0 0 2 2h20a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2zM6 6h20v4H6zm0 20v-14h20v14z"/><path d="M11 15h2v7h-2zm4-3h2v10h-2zm4 6h2v4h-2z"/></svg>
                  <svg v-else-if="item.icon === 'organization'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 2H10a2 2 0 0 0-2 2v2h-2a2 2 0 0 0-2 2v20a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM10 4h16v2H10zm12 24H6V8h2v18h14zm6-4H10V6h18z"/></svg>
                  <svg v-else-if="item.icon === 'user'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M16 4a5 5 0 1 1-5 5 5 5 0 0 1 5-5m0-2a7 7 0 1 0 7 7 7 7 0 0 0-7-7zm10 28h-2v-5a5 5 0 0 0-5-5H13a5 5 0 0 0-5 5v5H6v-5a7 7 0 0 1 7-7h6a7 7 0 0 1 7 7z"/></svg>
                  <svg v-else-if="item.icon === 'key'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M22 2a9 9 0 0 0-8.8 7.14L2 20.34V28a2 2 0 0 0 2 2h8v-2h2v-2h2v-2h2l1.62-1.62A9 9 0 1 0 22 2zm0 16a7 7 0 0 1-5.61-2.85l-1-1.34-1.63.26L12 14.34V22h-2v2H8v2H4v-4.34L13.14 11l.26-1.63-1.34-1A7 7 0 1 1 22 18z"/><circle cx="24" cy="8" r="2"/></svg>
                  <svg v-else-if="item.icon === 'repository'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 30H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h20a2 2 0 0 1 2 2v24a2 2 0 0 1-2 2zM6 4v24h20V4z"/><path d="M11 8h10v2H11zm0 4h10v2H11zm0 4h10v2H11zm0 4h7v2h-7z"/></svg>
                  <svg v-else-if="item.icon === 'patch'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 2H10a2 2 0 0 0-2 2v20a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2m0 22H10V4h16zm-12-6h8v2h-8zm0-4h8v2h-8zm0-4h8v2h-8zM6 26h16v2H6a2 2 0 0 1-2-2V8h2z"/></svg>
                  <svg v-else-if="item.icon === 'review'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="m13 24l-9-9l1.414-1.414L13 21.172L26.586 7.586L28 9z"/></svg>
                  <svg v-else-if="item.icon === 'issue'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M16 2a14 14 0 1 0 14 14A14 14 0 0 0 16 2m0 26a12 12 0 1 1 12-12 12 12 0 0 1-12 12"/><path d="M15 8h2v10h-2zm0 12h2v2h-2z"/></svg>
                  <svg v-else-if="item.icon === 'milestone'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="m20 2l-8 6h8v6l8-6zM4 28h24v2H4zM14 8v18h2V8z"/></svg>
                  <svg v-else-if="item.icon === 'pipeline'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M25.17 2.83a3 3 0 0 0-4.24 0l-1.34 1.34L13.17 6H6a2 2 0 0 0-2 2v2.17l4.59 4.59L4 19.34V28h8.66l4.58-4.59 4.59 4.59H24a2 2 0 0 0 2-2v-7.17l1.83-6.42 1.34-1.34a3 3 0 0 0 0-4.24zM24 26h-1.17l-4.59-4.59L22.83 17H24zM10.17 26H6v-4.17l4.59 4.59zM15 21.83L10.17 17l4.59-4.59 4.59 4.59zM24 15h-2.17l-4.59-4.59 4.59-4.59H24z"/></svg>
                  <svg v-else-if="item.icon === 'quality'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 2H6a2 2 0 0 0-2 2v24a2 2 0 0 0 2 2h20a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM6 4h18v2H6zm0 24V8h20v20z"/><path d="M10 12h12v2H10zm0 16h12v2H10zm0 4h7v2h-7z"/></svg>
                  <svg v-else-if="item.icon === 'knowledge'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M26 2H6a2 2 0 0 0-2 2v24a2 2 0 0 0 2 2h20a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM6 4h18v2H6zm0 24V8h20v20z"/><path d="M10 12h12v2H10zm0 4h12v2H10zm0 4h12v2H10z"/></svg>
                  <svg v-else-if="item.icon === 'audit'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M22 30H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10l8 8v10h-2V11h-7V4H6v24h16z"/><path d="M24 26a4 4 0 1 0-4-4 4 4 0 0 0 4 4m0 2a6 6 0 1 1 6-6 6 6 0 0 1-6 6M10 12h12v2H10zm0 4h7v2h-7zm0 4h7v2h-7z"/></svg>
                  <svg v-else-if="item.icon === 'settings'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M29.18 19.07l-1.68-.32A8.74 8.74 0 0 0 27 16.59l.54-1.63a1.58 1.58 0 0 0-.31-1.67l-1.52-1.52a1.58 1.58 0 0 0-1.67-.31l-1.63.54a8.74 8.74 0 0 0-2.16-.5l-.32-1.68A1.58 1.58 0 0 0 18.37 8.5h-2.14a1.58 1.58 0 0 0-1.56 1.32l-.32 1.68a8.74 8.74 0 0 0-2.16.5l-1.63-.54a1.58 1.58 0 0 0-1.67.31l-1.52 1.52a1.58 1.58 0 0 0-.31 1.67l.54 1.63a8.74 8.74 0 0 0-.5 2.16l-1.68.32A1.58 1.58 0 0 0 4 20.63v2.14a1.58 1.58 0 0 0 1.32 1.56l1.68.32a8.74 8.74 0 0 0 .5 2.16l-.54 1.63a1.58 1.58 0 0 0 .31 1.67l1.52 1.52a1.58 1.58 0 0 0 1.67.31l1.63-.54a8.74 8.74 0 0 0 2.16.5l.32 1.68A1.58 1.58 0 0 0 13.63 35.5h2.14a1.58 1.58 0 0 0 1.56-1.32l.32-1.68a8.74 8.74 0 0 0 2.16-.5l1.63.54a1.58 1.58 0 0 0 1.67-.31l1.52-1.52a1.58 1.58 0 0 0 .31-1.67l-.54-1.63a8.74 8.74 0 0 0 .5-2.16l1.68-.32A1.58 1.58 0 0 0 32 23.37v-2.14a1.58 1.58 0 0 0-1.32-1.56zM30 23.37a.58.58 0 0 1-.48.57l-1.89.36a.58.58 0 0 0-.45.45 7.74 7.74 0 0 1-.67 1.93.58.58 0 0 0 .1.63l1.21 1.21a.58.58 0 0 1 .11.61l-1.07 1.07a.58.58 0 0 1-.61.11l-1.21-1.21a.58.58 0 0 0-.63-.1 7.74 7.74 0 0 1-1.93.67.58.58 0 0 0-.45.45l-.36 1.89a.58.58 0 0 1-.57.48h-2.14a.58.58 0 0 1-.57-.48l-.36-1.89a.58.58 0 0 0-.45-.45 7.74 7.74 0 0 1-1.93-.67.58.58 0 0 0-.63.1l-1.21 1.21a.58.58 0 0 1-.61.11l-1.07-1.07a.58.58 0 0 1 .11-.61l1.21-1.21a.58.58 0 0 0 .1-.63 7.74 7.74 0 0 1-.67-1.93.58.58 0 0 0-.45-.45l-1.89-.36a.58.58 0 0 1-.48-.57v-2.14a.58.58 0 0 1 .48-.57l1.89-.36a.58.58 0 0 0 .45-.45 7.74 7.74 0 0 1 .67-1.93.58.58 0 0 0-.1-.63l-1.21-1.21a.58.58 0 0 1-.11-.61l1.07-1.07a.58.58 0 0 1 .61-.11l1.21 1.21a.58.58 0 0 0 .63.1 7.74 7.74 0 0 1 1.93-.67.58.58 0 0 0 .45-.45l.36-1.89a.58.58 0 0 1 .57-.48h2.14a.58.58 0 0 1 .57.48l.36 1.89a.58.58 0 0 0 .45.45 7.74 7.74 0 0 1 1.93.67.58.58 0 0 0 .63-.1l1.21-1.21a.58.58 0 0 1 .61-.11l1.07 1.07a.58.58 0 0 1-.11.61l-1.21 1.21a.58.58 0 0 0-.1.63 7.74 7.74 0 0 1 .67 1.93.58.58 0 0 0 .45.45l1.89.36a.58.58 0 0 1 .48.57z"/></svg>
                </div>
                <span class="text-[13px] font-medium">{{ item.name }}</span>
              </router-link>
            </div>
          </div>
        </nav>
      </div>

      <div class="p-3 border-t border-theme-border/50 bg-theme-bg-subtle/20">
        <div class="flex items-center justify-between gap-2 p-2 rounded-lg bg-theme-bg-lighter border border-theme-border/50 shadow-sm">
          <div class="flex items-center gap-2 overflow-hidden">
            <SxoAvatar :size="24" src="https://cube.elemecdn.com/0/88/03b0d39583f48206768a7534e55bcpng.png" />
            <div class="overflow-hidden">
              <p class="text-[12px] font-bold text-theme-text truncate leading-tight">Admin</p>
              <p class="text-[9px] text-theme-text-muted truncate leading-tight">@system</p>
            </div>
          </div>
          <SxoButton 
            variant="ghost"
            class="!text-red-500 hover:!bg-red-500 hover:!text-white !p-1.5 transition-all !h-auto" 
            @click="handleLogout"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M6 30h12v-2H6V4h12V2H6a2 2 0 0 0-2 2v24a2 2 0 0 0 2 2z"/><path d="M20.59 22L19.17 20.59L23.76 16l-4.59-4.59L20.59 10l6 6l-6 6z"/><path d="M10 15h15v2H10z"/></svg>
          </SxoButton>
        </div>
      </div>
    </aside>

    <div class="flex-1 flex flex-col overflow-hidden">
      <header class="h-16 bg-theme-bg-lighter/80 backdrop-blur-md border-b border-theme-border flex items-center justify-between px-8 sticky top-0 z-10">
        <div class="flex items-center gap-2">
          <nav class="flex items-center text-xs text-theme-text-muted">
            <span>控制面板</span>
            <template v-if="$route.path !== '/'">
              <span class="mx-2 opacity-30">/</span>
              <span class="text-theme-text font-medium">{{ $route.name || '当前页面' }}</span>
            </template>
          </nav>
        </div>
        <div class="flex items-center gap-4">
          <SxoButton variant="ghost" class="!bg-theme-bg-subtle !border-none !p-2 !h-auto rounded-full">
            <div class="i-carbon-notification" />
          </SxoButton>
          <SxoButton variant="ghost" class="!bg-theme-bg-subtle !border-none !p-2 !h-auto rounded-full">
            <div class="i-carbon-search" />
          </SxoButton>
        </div>
      </header>
      
      <main class="flex-1 overflow-y-auto bg-theme-bg/50 p-8">
        <router-view />
      </main>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--theme-border);
  border-radius: 10px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: var(--theme-text-muted);
}
</style>
