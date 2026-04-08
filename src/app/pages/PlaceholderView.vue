<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const pageName = computed(() => (route.meta.title as string)?.split(' - ')[0] || '页面')
</script>

<template>
  <div class="placeholder-page">
    <div class="placeholder-card">
      <i class="pi pi-wrench placeholder-icon"></i>
      <h2>{{ pageName }}</h2>
      <p>该页面正在建设中…</p>
    </div>
  </div>
</template>

<style scoped>
.placeholder-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
}

.placeholder-card {
  text-align: center;
  padding: 40px;
  opacity: 0.5;
}

.placeholder-icon {
  font-size: 2.5rem;
  margin-bottom: 12px;
  display: block;
}

.placeholder-card h2 {
  margin: 0 0 8px;
  font-size: 1.2rem;
}

.placeholder-card p {
  margin: 0;
  font-size: 0.88rem;
  opacity: 0.7;
}
</style>
