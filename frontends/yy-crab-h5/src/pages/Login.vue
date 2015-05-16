<script setup lang="ts">
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { SxoButton, SxoInput } from '@sxo/vue'

const router = useRouter()
const loading = ref(false)
const loginForm = ref({
    username: '',
    password: ''
})

const handleLogin = async () => {
    if (!loginForm.value.username || !loginForm.value.password) {
        return
    }
  
    loading.value = true
    setTimeout(() => {
        sessionStorage.setItem('ys-admin-token', 'mock-token-' + Date.now())
        loading.value = false
        router.push('/')
    }, 800)
}
</script>

<template>
  <div class="flex min-h-screen bg-theme-bg overflow-hidden">
    <!-- Left Side: Branding & Philosophy -->
    <div class="hidden lg:flex lg:w-3/5 relative bg-theme-primary items-center justify-center p-12 overflow-hidden">
      <!-- Decorative Background Elements -->
      <div class="absolute top-[-10%] left-[-10%] w-[60%] h-[60%] rounded-full bg-white opacity-10 blur-[120px]" />
      <div class="absolute bottom-[-10%] right-[-10%] w-[50%] h-[50%] rounded-full bg-theme-secondary opacity-20 blur-[100px]" />
      
      <div class="relative z-10 max-w-2xl text-white">
        <h1 class="text-6xl font-bold mb-6 tracking-tight">YS Platform</h1>
        <p class="text-2xl font-light opacity-90 mb-12 leading-relaxed">
          基于补丁代数理论的下一代软件开发平台
        </p>
        
        <div class="grid grid-cols-2 gap-8 text-left">
          <div class="bg-white/10 backdrop-blur-md p-6 rounded-2xl border border-white/20">
            <h3 class="text-lg font-bold mb-2">补丁代数 (Patch Algebra)</h3>
            <p class="text-sm opacity-70">形式化定义补丁变换，实现无冲突重排与确定性历史。</p>
          </div>
          <div class="bg-white/10 backdrop-blur-md p-6 rounded-2xl border border-white/20">
            <h3 class="text-lg font-bold mb-2">内容寻址 (Content Addressed)</h3>
            <p class="text-sm opacity-70">提交哈希仅基于内容，实现全局去重与极致溯源。</p>
          </div>
          <div class="bg-white/10 backdrop-blur-md p-6 rounded-2xl border border-white/20">
            <h3 class="text-lg font-bold mb-2">交换收敛 (Convergence)</h3>
            <p class="text-sm opacity-70">分布式环境下的操作最终收敛于数学确定的状态。</p>
          </div>
          <div class="bg-white/10 backdrop-blur-md p-6 rounded-2xl border border-white/20">
            <h3 class="text-lg font-bold mb-2">智能集成 (Native AI)</h3>
            <p class="text-sm opacity-70">原生集成补丁建议与影响分析，重塑开发者体验。</p>
          </div>
        </div>
      </div>

      <!-- Bottom Tagline -->
      <div class="absolute bottom-12 left-12 text-white/50 text-sm font-mono">
        VERSION 1.0.0-BETA // MATHEMATICAL RIGOR FIRST
      </div>
    </div>

    <!-- Right Side: Login Form -->
    <div class="w-full lg:w-2/5 flex items-center justify-center p-8 bg-theme-bg-lighter relative">
      <div class="w-full max-w-md">
        <div class="mb-12">
          <h2 class="text-3xl font-bold text-theme-text mb-3">管理员登录</h2>
          <p class="text-theme-text-muted">请输入您的凭据以访问控制面板</p>
        </div>

        <form class="space-y-6" @submit.prevent="handleLogin" @keyup.enter="handleLogin">
          <div class="space-y-2">
            <label class="block text-sm font-medium text-theme-text-muted ml-1">管理员账号</label>
            <div class="relative group">
              <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none text-theme-text-muted group-focus-within:text-theme-primary transition-colors">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
              </div>
              <SxoInput 
                v-model="loginForm.username" 
                placeholder="Username / Email"
                class="!pl-11 !h-14 !rounded-xl !bg-theme-bg-subtle !border-theme-border focus-within:!border-theme-primary focus-within:!bg-theme-bg-lighter transition-all"
              />
            </div>
          </div>

          <div class="space-y-2">
            <label class="block text-sm font-medium text-theme-text-muted ml-1">安全密码</label>
            <div class="relative group">
              <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none text-theme-text-muted group-focus-within:text-theme-primary transition-colors">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
              </div>
              <SxoInput 
                v-model="loginForm.password" 
                type="password" 
                placeholder="Enter your password"
                class="!pl-11 !h-14 !rounded-xl !bg-theme-bg-subtle !border-theme-border focus-within:!border-theme-primary focus-within:!bg-theme-bg-lighter transition-all"
              />
            </div>
          </div>
          
          <div class="flex items-center justify-between mb-8">
            <label class="flex items-center gap-2 cursor-pointer group">
              <input type="checkbox" class="w-4 h-4 rounded border-theme-border text-theme-primary focus:ring-theme-primary/20 transition-all cursor-pointer">
              <span class="text-sm text-theme-text-muted group-hover:text-theme-text transition-colors">记住此设备</span>
            </label>
            <SxoButton variant="ghost" size="sm" class="!text-theme-primary hover:!bg-theme-primary/10 !px-2 !font-medium">
              忘记密码？
            </SxoButton>
          </div>

          <SxoButton 
            variant="primary"
            class="w-full !h-14 !text-lg !rounded-xl shadow-lg shadow-theme-primary/20 font-bold"
            :loading="loading"
            @click="handleLogin"
          >
            源神启动！
          </SxoButton>
        </form>

        <div class="mt-12 text-center">
          <p class="text-xs text-theme-text-muted uppercase tracking-widest">
            &copy; 2026 YS PLATFORM FOUNDATION
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 移除 Element Plus 相关样式，由 Tailwind 和 SXO 组件处理 */
</style>

<route lang="yaml">
meta:
  layout: Blank
</route>
