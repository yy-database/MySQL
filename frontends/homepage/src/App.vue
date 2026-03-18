<template>
  <div class="min-h-screen bg-white">
    <header class="bg-white border-b border-gray-100 sticky top-0 z-50 shadow-sm">
      <div class="container mx-auto px-6">
        <div class="flex items-center justify-between h-20">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 bg-[#2496ED] rounded-lg flex items-center justify-center">
              <span class="text-white font-bold text-xl">D</span>
            </div>
            <span class="text-2xl font-bold text-gray-900">Docker Crab</span>
          </div>
          
          <nav class="flex items-center gap-10">
            <button 
              @click="currentSection = 'home'"
              :class="[
                'px-4 py-2 font-medium transition-all relative',
                currentSection === 'home' ? 'text-[#2496ED]' : 'text-gray-700 hover:text-[#2496ED]'
              ]"
            >
              Home
              <span v-if="currentSection === 'home'" class="absolute bottom-0 left-0 w-full h-0.5 bg-[#2496ED]"></span>
            </button>
            <button 
              @click="currentSection = 'features'"
              :class="[
                'px-4 py-2 font-medium transition-all relative',
                currentSection === 'features' ? 'text-[#2496ED]' : 'text-gray-700 hover:text-[#2496ED]'
              ]"
            >
              Features
              <span v-if="currentSection === 'features'" class="absolute bottom-0 left-0 w-full h-0.5 bg-[#2496ED]"></span>
            </button>
            <button 
              @click="currentSection = 'download'"
              :class="[
                'px-4 py-2 font-medium transition-all relative',
                currentSection === 'download' ? 'text-[#2496ED]' : 'text-gray-700 hover:text-[#2496ED]'
              ]"
            >
              Download
              <span v-if="currentSection === 'download'" class="absolute bottom-0 left-0 w-full h-0.5 bg-[#2496ED]"></span>
            </button>
            <button 
              @click="currentSection = 'docs'"
              :class="[
                'px-4 py-2 font-medium transition-all relative',
                currentSection === 'docs' ? 'text-[#2496ED]' : 'text-gray-700 hover:text-[#2496ED]'
              ]"
            >
              Documentation
              <span v-if="currentSection === 'docs'" class="absolute bottom-0 left-0 w-full h-0.5 bg-[#2496ED]"></span>
            </button>
          </nav>
          
          <div>
            <button class="px-6 py-2 bg-[#2496ED] text-white rounded-lg font-medium hover:bg-[#1a7cc8] transition-all shadow-md">
              Get Started
            </button>
          </div>
        </div>
      </div>
    </header>

    <div v-if="currentSection === 'home'" class="bg-gradient-to-br from-gray-50 to-gray-100 py-24">
      <div class="container mx-auto px-6">
        <div class="max-w-5xl mx-auto text-center">
          <h1 class="text-6xl font-extrabold text-gray-900 mb-6 leading-tight">
            基于 Rust 的下一代 Docker 工具平台
          </h1>
          <p class="text-2xl text-gray-600 mb-12 max-w-3xl mx-auto leading-relaxed">
            Docker Crab 采用 Rust 语言构建，提供高性能、安全可靠的容器管理解决方案，为企业级应用提供极致的运行效率和稳定性。
          </p>
          <div class="flex gap-6 justify-center">
            <button 
              @click="currentSection = 'download'"
              class="px-8 py-4 bg-[#2496ED] text-white rounded-lg font-semibold text-lg hover:bg-[#1a7cc8] transition-all shadow-lg"
            >
              立即下载
            </button>
            <button 
              @click="currentSection = 'features'"
              class="px-8 py-4 border border-gray-300 text-gray-700 rounded-lg font-semibold text-lg hover:bg-gray-50 transition-all"
            >
              了解功能
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="currentSection === 'home'" class="py-20">
      <div class="container mx-auto px-6">
        <div class="text-center mb-16">
          <h2 class="text-4xl font-bold text-gray-900 mb-4">核心优势</h2>
          <p class="text-xl text-gray-600 max-w-3xl mx-auto">
            Docker Crab 结合 Rust 语言的特性，为您的容器管理带来革命性的体验
          </p>
        </div>
        
        <div class="grid grid-cols-3 gap-8">
          <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
            <div class="w-14 h-14 bg-[#2496ED] rounded-lg flex items-center justify-center mb-6">
              <svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
              </svg>
            </div>
            <h3 class="text-2xl font-bold text-gray-900 mb-3">极致性能</h3>
            <p class="text-gray-600">Rust 语言的零成本抽象和高性能特性，使 Docker Crab 在处理大量容器时表现卓越，响应速度提升 30% 以上。</p>
          </div>
          <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
            <div class="w-14 h-14 bg-[#2496ED] rounded-lg flex items-center justify-center mb-6">
              <svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"></path>
              </svg>
            </div>
            <h3 class="text-2xl font-bold text-gray-900 mb-3">内存安全</h3>
            <p class="text-gray-600">Rust 的所有权系统和借用检查器确保内存安全，消除了常见的内存错误和安全漏洞，为容器管理提供更可靠的保障。</p>
          </div>
          <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
            <div class="w-14 h-14 bg-[#2496ED] rounded-lg flex items-center justify-center mb-6">
              <svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
            </div>
            <h3 class="text-2xl font-bold text-gray-900 mb-3">企业级可靠性</h3>
            <p class="text-gray-600">基于 Rust 的稳定性和 Docker 的标准化，Docker Crab 为企业级应用提供高可靠性和一致性的容器管理体验。</p>
          </div>
        </div>
      </div>
    </div>

    <div v-if="currentSection === 'features'" class="py-20">
      <div class="container mx-auto px-6">
        <div class="text-center mb-16">
          <h2 class="text-4xl font-bold text-gray-900 mb-4">强大功能</h2>
          <p class="text-xl text-gray-600 max-w-3xl mx-auto">
            Docker Crab 提供全面的容器管理功能，满足企业级应用的各种需求
          </p>
        </div>
        
        <div class="grid grid-cols-2 gap-12">
          <div class="flex gap-6">
            <div class="w-16 h-16 bg-[#2496ED] rounded-lg flex items-center justify-center flex-shrink-0">
              <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"></path>
              </svg>
            </div>
            <div>
              <h3 class="text-2xl font-bold text-gray-900 mb-3">容器管理</h3>
              <p class="text-gray-600 mb-4">高效管理容器生命周期，支持批量操作、健康检查和自动重启，提供直观的监控面板。</p>
              <ul class="text-gray-600 space-y-2">
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  实时容器状态监控
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  批量容器操作
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  容器日志管理
                </li>
              </ul>
            </div>
          </div>
          <div class="flex gap-6">
            <div class="w-16 h-16 bg-[#2496ED] rounded-lg flex items-center justify-center flex-shrink-0">
              <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"></path>
              </svg>
            </div>
            <div>
              <h3 class="text-2xl font-bold text-gray-900 mb-3">镜像管理</h3>
              <p class="text-gray-600 mb-4">智能镜像管理，支持镜像拉取、推送、构建和缓存，优化镜像存储和传输。</p>
              <ul class="text-gray-600 space-y-2">
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  镜像版本控制
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  镜像分层管理
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  镜像构建自动化
                </li>
              </ul>
            </div>
          </div>
          <div class="flex gap-6">
            <div class="w-16 h-16 bg-[#2496ED] rounded-lg flex items-center justify-center flex-shrink-0">
              <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 18h.01M7 21h10a2 2 0 002-2V5a2 2 0 00-2-2H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path>
              </svg>
            </div>
            <div>
              <h3 class="text-2xl font-bold text-gray-900 mb-3">网络管理</h3>
              <p class="text-gray-600 mb-4">灵活的网络配置，支持自定义网络、端口映射和网络隔离，确保容器间通信安全高效。</p>
              <ul class="text-gray-600 space-y-2">
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  自定义网络创建
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  端口映射管理
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  网络安全策略
                </li>
              </ul>
            </div>
          </div>
          <div class="flex gap-6">
            <div class="w-16 h-16 bg-[#2496ED] rounded-lg flex items-center justify-center flex-shrink-0">
              <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
              </svg>
            </div>
            <div>
              <h3 class="text-2xl font-bold text-gray-900 mb-3">存储管理</h3>
              <p class="text-gray-600 mb-4">高效的存储管理，支持卷管理、数据持久化和存储优化，确保数据安全可靠。</p>
              <ul class="text-gray-600 space-y-2">
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  卷创建和管理
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  数据持久化
                </li>
                <li class="flex items-center gap-2">
                  <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                  </svg>
                  存储性能优化
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="currentSection === 'download'" class="py-20">
      <div class="container mx-auto px-6">
        <div class="max-w-4xl mx-auto">
          <h1 class="text-4xl font-bold text-gray-900 mb-8 text-center">下载 Docker Crab</h1>
          <p class="text-xl text-gray-600 mb-12 text-center max-w-2xl mx-auto">
            选择适合您操作系统的版本，开始体验基于 Rust 的高性能容器管理解决方案
          </p>
          
          <div class="grid grid-cols-2 gap-8">
            <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
              <div class="flex items-center gap-4 mb-6">
                <div class="w-12 h-12 bg-blue-50 rounded-lg flex items-center justify-center">
                  <svg class="w-6 h-6 text-blue-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd"></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-xl font-bold text-gray-900">Windows</h3>
                  <p class="text-gray-500">Windows 10/11</p>
                </div>
              </div>
              <button class="w-full py-3 bg-[#2496ED] text-white rounded-lg font-semibold hover:bg-[#1a7cc8] transition-all">
                下载 Windows 版本
              </button>
            </div>
            
            <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
              <div class="flex items-center gap-4 mb-6">
                <div class="w-12 h-12 bg-gray-50 rounded-lg flex items-center justify-center">
                  <svg class="w-6 h-6 text-gray-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd"></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-xl font-bold text-gray-900">macOS</h3>
                  <p class="text-gray-500">macOS 11+</p>
                </div>
              </div>
              <button class="w-full py-3 bg-[#2496ED] text-white rounded-lg font-semibold hover:bg-[#1a7cc8] transition-all">
                下载 macOS 版本
              </button>
            </div>
            
            <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
              <div class="flex items-center gap-4 mb-6">
                <div class="w-12 h-12 bg-yellow-50 rounded-lg flex items-center justify-center">
                  <svg class="w-6 h-6 text-yellow-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M5.05 4.05a7 7 0 119.9 9.9L10 18.9l-4.95-4.95a7 7 0 010-9.9zM10 11a2 2 0 100-4 2 2 0 000 4z" clip-rule="evenodd"></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-xl font-bold text-gray-900">Linux</h3>
                  <p class="text-gray-500">Ubuntu, Fedora, etc.</p>
                </div>
              </div>
              <button class="w-full py-3 bg-[#2496ED] text-white rounded-lg font-semibold hover:bg-[#1a7cc8] transition-all">
                下载 Linux 版本
              </button>
            </div>
            
            <div class="p-8 border border-gray-200 rounded-lg hover:shadow-lg transition-all">
              <div class="flex items-center gap-4 mb-6">
                <div class="w-12 h-12 bg-gray-50 rounded-lg flex items-center justify-center">
                  <svg class="w-6 h-6 text-gray-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M4 2a2 2 0 00-2 2v11a3 3 0 106 0V4a2 2 0 00-2-2H4zm1 14a1 1 0 100-2 1 1 0 000 2zm5-15a2 2 0 012-2h1a2 2 0 012 2v12a3 3 0 11-6 0V1z" clip-rule="evenodd"></path>
                  </svg>
                </div>
                <div>
                  <h3 class="text-xl font-bold text-gray-900">源码</h3>
                  <p class="text-gray-500">GitHub 仓库</p>
                </div>
              </div>
              <button class="w-full py-3 bg-gray-800 text-white rounded-lg font-semibold hover:bg-gray-700 transition-all">
                查看 GitHub
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="currentSection === 'docs'" class="h-[calc(100vh-5rem)] py-0">
      <div class="container mx-auto px-6 h-full">
        <div class="flex gap-6 h-full">
          <aside class="w-80 flex-shrink-0">
            <div class="border border-gray-200 rounded-lg p-6 sticky top-24 bg-white h-full overflow-y-auto">
              <h2 class="text-xl font-bold text-gray-900 mb-6">文档导航</h2>
              <nav v-if="docTree.length > 0">
                <div v-for="node in docTree" :key="node.id" class="mb-1">
                  <DocTreeNode :node="node" :current-path="currentDocPath" @select="selectDoc" />
                </div>
              </nav>
              <div v-else class="text-gray-500 text-sm">
                加载文档中...
              </div>
            </div>
          </aside>

          <main class="flex-1 h-full">
            <div class="border border-gray-200 rounded-lg p-8 bg-white h-full overflow-y-auto">
              <div v-if="currentDoc">
                <h2 class="text-3xl font-bold mb-6 text-gray-900">{{ currentDoc.title }}</h2>
                <div class="prose max-w-none">
                  <MarkdownViewer :content="currentDocContent" />
                </div>
              </div>
              <div v-else class="text-gray-500 text-center py-16">
                <p class="text-lg">请从左侧选择文档查看</p>
              </div>
            </div>
          </main>
        </div>
      </div>
    </div>

    <footer class="bg-gray-800 text-white py-12">
      <div class="container mx-auto px-4">
        <div class="grid grid-cols-4 gap-8">
          <div>
            <div class="flex items-center gap-3 mb-4">
              <div class="w-8 h-8 bg-[#2496ED] rounded-lg flex items-center justify-center">
                <span class="text-white font-bold text-sm">D</span>
              </div>
              <span class="text-xl font-bold text-white">Docker Crab</span>
            </div>
            <p class="text-gray-400 text-sm">
              基于 Rust 构建的下一代 Docker 工具平台，提供高性能、安全可靠的容器管理解决方案。
            </p>
          </div>
          <div>
            <h3 class="text-lg font-semibold mb-4">产品</h3>
            <ul class="space-y-2 text-gray-400 text-sm">
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">容器管理</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">镜像管理</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">网络管理</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">存储管理</a></li>
            </ul>
          </div>
          <div>
            <h3 class="text-lg font-semibold mb-4">资源</h3>
            <ul class="space-y-2 text-gray-400 text-sm">
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">文档</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">教程</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">API 参考</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">社区</a></li>
            </ul>
          </div>
          <div>
            <h3 class="text-lg font-semibold mb-4">关于我们</h3>
            <ul class="space-y-2 text-gray-400 text-sm">
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">团队</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">博客</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">联系我们</a></li>
              <li><a href="#" class="hover:text-[#2496ED] transition-colors">GitHub</a></li>
            </ul>
          </div>
        </div>
        <div class="border-t border-gray-700 mt-8 pt-8 flex flex-col md:flex-row justify-between items-center">
          <p class="text-gray-500 text-sm">
            © 2026 Docker Crab. All rights reserved.
          </p>
          <div class="flex gap-4 mt-4 md:mt-0">
            <a href="#" class="text-gray-400 hover:text-[#2496ED] transition-colors">
              <span class="text-lg">🌐</span>
            </a>
            <a href="#" class="text-gray-400 hover:text-[#2496ED] transition-colors">
              <span class="text-lg">🐙</span>
            </a>
            <a href="#" class="text-gray-400 hover:text-[#2496ED] transition-colors">
              <span class="text-lg">🐦</span>
            </a>
            <a href="#" class="text-gray-400 hover:text-[#2496ED] transition-colors">
              <span class="text-lg">📧</span>
            </a>
          </div>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { loadDocs, getDocContent, type DocNode } from "@/docs";
import MarkdownViewer from "@/components/MarkdownViewer.vue";
import DocTreeNode from "@/components/DocTreeNode.vue";

const currentSection = ref<"home" | "download" | "docs">("home");
const docTree = ref<DocNode[]>([]);
const currentDocPath = ref<string>("");
const currentDoc = ref<DocNode | null>(null);
const currentDocContent = ref<string>("");

async function init() {
    console.log("Initializing docs...");
    docTree.value = await loadDocs();
    console.log("Loaded docTree:", docTree.value);
    if (docTree.value.length > 0) {
        const firstDoc = findFirstDoc(docTree.value[0]);
        if (firstDoc) {
            selectDoc(firstDoc);
        }
    }
}

function findFirstDoc(node: DocNode): DocNode | null {
    if (!node.isDirectory && !node.children?.length) {
        return node;
    }
    if (node.children && node.children.length > 0) {
        return findFirstDoc(node.children[0]);
    }
    return null;
}

async function selectDoc(node: DocNode) {
    currentDocPath.value = node.path;
    currentDoc.value = node;
    currentDocContent.value = await getDocContent(node.path);
}

onMounted(() => {
    init();
});
</script>
