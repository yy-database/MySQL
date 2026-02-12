---
layout: home
---

<script setup>
import { onMounted } from 'vue'
import { useRouter } from 'vitepress'

const router = useRouter()

onMounted(() => {
  const lang = navigator.language.startsWith('zh') ? 'zh' : 'en'
  router.go(`/${lang}/`)
})
</script>
