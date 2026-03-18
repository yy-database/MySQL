<script setup lang="ts">
import { SxoButton } from '@sxo/vue'
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
    <div v-for="i in 4" :key="i" class="theme-card">
      <div class="text-theme-text-muted text-sm mb-2">数据指标 {{ i }}</div>
      <div class="text-2xl font-bold">{{ Math.floor(Math.random() * 1000) }}</div>
      <div class="mt-4 text-xs text-green-500">↑ 12% 较上周</div>
    </div>
  </div>

  <div class="mt-8 theme-card">
    <h3 class="text-lg font-bold mb-4">待处理事项</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-left border-collapse">
        <thead>
          <tr class="border-b border-theme-border text-theme-text-muted text-xs uppercase tracking-wider">
            <th class="px-4 py-3 font-semibold">ID</th>
            <th class="px-4 py-3 font-semibold">任务标题</th>
            <th class="px-4 py-3 font-semibold">状态</th>
            <th class="px-4 py-3 font-semibold text-right">操作</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-theme-border">
          <!-- Empty state -->
          <tr>
            <td colspan="4" class="px-4 py-12 text-center text-theme-text-muted">
              <div class="flex flex-col items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="opacity-20"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
                <span class="text-sm">暂无数据</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<route lang="yaml">
meta:
  name: 控制台
</route>

