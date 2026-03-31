<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import TitleBar from '../components/TitleBar.vue'
import { useUserStore } from '../stores/user'
import { logger } from '../utils/logger'
import { activateLoginWindow } from '../utils/window'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const mainWindow = getCurrentWindow()

onMounted(async () => {
  // #region agent log
  fetch('http://127.0.0.1:7486/ingest/637909e2-8eda-4b2c-af00-1f426bec300c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'23ef92'},body:JSON.stringify({sessionId:'23ef92',location:'MainLayout.vue:onMounted',message:'MainLayout onMounted fired',data:{windowLabel:mainWindow.label},timestamp:Date.now()})}).catch(()=>{});
  // #endregion
  await mainWindow.show()
  await mainWindow.setFocus()
  // #region agent log
  fetch('http://127.0.0.1:7486/ingest/637909e2-8eda-4b2c-af00-1f426bec300c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'23ef92'},body:JSON.stringify({sessionId:'23ef92',location:'MainLayout.vue:onMounted:done',message:'MainLayout show/focus done',data:{},timestamp:Date.now()})}).catch(()=>{});
  // #endregion
})

const displayName = computed(() => userStore.userInfo?.username || '用户')
const menuItems = [
  { label: '仪表盘', icon: 'pi pi-home', path: '/main/dashboard' },
]

const pageTitle = computed(() => {
  if (route.path === '/main/dashboard') return '仪表盘'
  return '主页'
})

async function handleNavigate(path: string) {
  logger.log('main-layout', '点击菜单导航', { path })
  await router.push(path)
}

async function handleLogout() {
  logger.log('main-layout', '用户退出登录')
  userStore.logout()
  await activateLoginWindow()
}
</script>

<template>
  <div class="layout-root">
    <TitleBar variant="full" :title="pageTitle" />

    <div class="layout-body">
      <aside class="sidebar">
        <div class="sidebar-user">
          <div class="sidebar-avatar">{{ displayName.charAt(0) }}</div>
          <div class="sidebar-user-meta">
            <strong>{{ displayName }}</strong>
            <span>{{ userStore.isLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
        </div>

        <nav class="sidebar-nav">
          <button
            v-for="item in menuItems"
            :key="item.path"
            type="button"
            class="nav-item"
            :class="{ active: route.path === item.path }"
            @click="handleNavigate(item.path)"
          >
            <i :class="item.icon"></i>
            <span>{{ item.label }}</span>
          </button>
        </nav>

        <button type="button" class="logout-btn" @click="handleLogout">
          <i class="pi pi-sign-out"></i>
          <span>退出登录</span>
        </button>
      </aside>

      <section class="main-area">
        <div class="content-header">
          <div class="header-left">
            <div class="page-title">{{ pageTitle }}</div>
            <div class="page-subtitle">欢迎回来，开始今天的任务吧</div>
          </div>
          <div class="header-right">
            <div class="user-card">
              <div class="user-avatar">{{ displayName.charAt(0) }}</div>
              <div class="user-meta">
                <strong>{{ displayName }}</strong>
                <span>{{ userStore.userInfo?.phone || userStore.userInfo?.email || '当前账号' }}</span>
              </div>
            </div>
          </div>
        </div>

        <main class="content-shell">
          <RouterView />
        </main>
      </section>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.layout-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #eef2f7;
  animation: page-fade-in 0.35s ease-out;
}

.layout-body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.sidebar {
  width: 240px;
  flex-shrink: 0;
  background: #102136;
  color: #e2e8f0;
  display: flex;
  flex-direction: column;
  padding: 0 12px 12px;
}

.sidebar-user {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  margin-bottom: 12px;
}

.sidebar-avatar {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.95rem;
  flex-shrink: 0;
}

.sidebar-user-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.sidebar-user-meta strong {
  color: #fff;
  font-size: 0.92rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-user-meta span {
  color: #94a3b8;
  font-size: 0.75rem;
}

.sidebar-nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  height: 42px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: #cbd5e1;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 14px;
  cursor: pointer;
  font-size: 0.88rem;
  transition: all 0.15s ease;
  width: 100%;
  text-align: left;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(45, 212, 191, 0.22), rgba(13, 148, 136, 0.32));
  color: #fff;
  box-shadow: inset 0 0 0 1px rgba(94, 234, 212, 0.12);
}

.logout-btn {
  height: 42px;
  border: none;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px;
  cursor: pointer;
  font-size: 0.85rem;
  transition: all 0.15s;
  width: 100%;
  margin-top: 8px;
}

.logout-btn:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #fca5a5;
}

.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.content-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 64px;
  padding: 0 24px;
  background: #fff;
  border-bottom: 1px solid #e2e8f0;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.page-title {
  font-size: 1rem;
  font-weight: 700;
  color: #0f172a;
}

.page-subtitle {
  font-size: 0.78rem;
  color: #64748b;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.user-card {
  display: flex;
  align-items: center;
  gap: 10px;
}

.user-avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.85rem;
}

.user-meta {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.user-meta strong {
  font-size: 0.88rem;
  color: #0f172a;
}

.user-meta span {
  font-size: 0.72rem;
  color: #64748b;
}

.content-shell {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 18px;
}
</style>
