<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import { showDialog } from '@/utils/dialog'
import { exitApp } from '@/utils/window'

defineProps<{
  variant?: 'full' | 'minimal'
  title?: string
}>()

const APP_NAME = '青拾'
const APP_VERSION = 'V0.1.0'

const appWindow = getCurrentWindow()
const isMaximized = ref(false)
const showMenu = ref(false)
const closeMode = ref<'exit' | 'minimize'>((localStorage.getItem('close_mode') as 'exit' | 'minimize') || 'exit')
const autoStartEnabled = ref(false)

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized()
  await appWindow.onResized(async () => {
    isMaximized.value = await appWindow.isMaximized()
  })
  document.addEventListener('click', closeMenu)
  try {
    autoStartEnabled.value = await isEnabled()
  } catch { /* ignore */ }
})

onUnmounted(() => {
  document.removeEventListener('click', closeMenu)
})

function closeMenu() {
  showMenu.value = false
}

function toggleMenu(e: MouseEvent) {
  e.stopPropagation()
  showMenu.value = !showMenu.value
}

async function handleMinimize() {
  await appWindow.minimize()
}

async function handleToggleMaximize() {
  await appWindow.toggleMaximize()
}

async function handleClose() {
  if (closeMode.value === 'minimize') {
    await appWindow.hide()
  } else {
    await exitApp()
  }
}

function onRefresh() {
  showMenu.value = false
  window.location.reload()
}

function onClearCache() {
  showMenu.value = false
  const keepKeys = ['saved_accounts', 'token', 'userInfo', 'close_mode']
  const saved: Record<string, string> = {}
  keepKeys.forEach(k => {
    const v = localStorage.getItem(k)
    if (v) saved[k] = v
  })
  sessionStorage.clear()
  localStorage.clear()
  Object.entries(saved).forEach(([k, v]) => localStorage.setItem(k, v))
  window.location.reload()
}

async function setAutoStart(enabled: boolean) {
  showMenu.value = false
  try {
    if (enabled) {
      await enable()
    } else {
      await disable()
    }
    autoStartEnabled.value = await isEnabled()
  } catch { /* ignore */ }
}

function setCloseMode(mode: 'exit' | 'minimize') {
  showMenu.value = false
  closeMode.value = mode
  localStorage.setItem('close_mode', mode)
}

async function onAbout() {
  showMenu.value = false
  await showDialog({
    title: '关于',
    message: `${APP_NAME} ${APP_VERSION}\n© 2024-2026 ${APP_NAME}`,
  })
}

async function onExit() {
  showMenu.value = false
  await exitApp()
}
</script>

<template>
  <header class="app-titlebar" style="-webkit-app-region: drag">
    <div v-if="variant === 'full'" class="tb-left">
      <div class="tb-brand-icon">青</div>
      <span class="tb-name">{{ APP_NAME }}</span>
      <span class="tb-version">{{ APP_VERSION }}</span>
      <template v-if="title">
        <span class="tb-sep">|</span>
        <span class="tb-page">{{ title }}</span>
      </template>
    </div>

    <div class="tb-right" style="-webkit-app-region: no-drag">
      <div class="app-menu-wrap">
        <button class="win-btn app-menu-trigger" title="菜单" @click="toggleMenu($event)">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><line x1="4" y1="6" x2="20" y2="6"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="18" x2="20" y2="18"/></svg>
        </button>
        <div v-if="showMenu" class="app-menu-dropdown" @click.stop>
          <button class="app-menu-item" @click="onRefresh">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
            刷新界面
          </button>
          <button class="app-menu-item" @click="onClearCache">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/></svg>
            清理缓存
          </button>
          <div class="app-menu-divider" />
          <div class="app-menu-has-sub">
            <button class="app-menu-item">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v4"/><path d="M12 18v4"/><path d="M4.93 4.93l2.83 2.83"/><path d="M16.24 16.24l2.83 2.83"/><path d="M2 12h4"/><path d="M18 12h4"/><path d="M4.93 19.07l2.83-2.83"/><path d="M16.24 7.76l2.83-2.83"/></svg>
              开机自启
              <span class="sub-arrow">›</span>
            </button>
            <div class="app-sub-menu">
              <button class="app-menu-item" @click="setAutoStart(true)">
                <span class="check-mark">{{ autoStartEnabled ? '✓' : '' }}</span>
                开启
              </button>
              <button class="app-menu-item" @click="setAutoStart(false)">
                <span class="check-mark">{{ !autoStartEnabled ? '✓' : '' }}</span>
                关闭
              </button>
            </div>
          </div>
          <div class="app-menu-has-sub">
            <button class="app-menu-item">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
              关闭设置
              <span class="sub-arrow">›</span>
            </button>
            <div class="app-sub-menu">
              <button class="app-menu-item" @click="setCloseMode('exit')">
                <span class="check-mark">{{ closeMode === 'exit' ? '✓' : '' }}</span>
                直接退出
              </button>
              <button class="app-menu-item" @click="setCloseMode('minimize')">
                <span class="check-mark">{{ closeMode === 'minimize' ? '✓' : '' }}</span>
                最小化到后台
              </button>
            </div>
          </div>
          <div class="app-menu-divider" />
          <button class="app-menu-item" @click="onAbout">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
            关于
          </button>
          <button class="app-menu-item app-menu-item--danger" @click="onExit">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
            退出程序
          </button>
        </div>
      </div>

      <button class="win-btn" title="最小化" @click="handleMinimize">–</button>
      <button class="win-btn" title="最大化/还原" @click="handleToggleMaximize">
        {{ isMaximized ? '❐' : '▢' }}
      </button>
      <button class="win-btn win-btn--close" title="关闭" @click="handleClose">✕</button>
    </div>
  </header>
</template>

<style scoped>
.app-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
  padding: 0 14px;
  background: var(--qs-bg-gradient);
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
  color: #ccfbf1;
  font-size: 0.92rem;
  flex-shrink: 0;
  z-index: 200;
}

.tb-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.tb-brand-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.18);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 0.95rem;
}

.tb-name {
  font-weight: 650;
  color: #fff;
  font-size: 1.05rem;
  line-height: 1;
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.12);
}

.tb-version {
  color: #fef3c7;
  font-size: 0.72rem;
  font-weight: 600;
  padding: 2px 8px;
  background: rgba(255, 255, 255, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
}

.tb-sep {
  color: rgba(255, 255, 255, 0.4);
  font-size: 0.9rem;
}

.tb-page {
  color: #f0fdfa;
  font-size: 0.92rem;
  font-weight: 500;
}

.tb-right {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.win-btn {
  width: 38px;
  height: 30px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 600;
  line-height: 1;
  color: rgba(255, 255, 255, 0.85);
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  cursor: pointer;
  transition: all 0.15s;
}

.win-btn:hover {
  background: rgba(255, 255, 255, 0.22);
  color: #fff;
}

.win-btn--close:hover {
  background: #ef4444;
  color: white;
  border-color: #ef4444;
}

.app-menu-wrap {
  position: relative;
}

.app-menu-trigger {
  font-size: 14px;
}

.app-menu-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  min-width: 168px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  box-shadow: 0 10px 36px rgba(15, 23, 42, 0.18);
  padding: 6px 0;
  z-index: 500;
}

.app-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 16px;
  border: none;
  background: none;
  color: #334155;
  font-size: 0.86rem;
  cursor: pointer;
  transition: background 0.12s;
  white-space: nowrap;
}

.app-menu-item:hover {
  background: #f1f5f9;
}

.app-menu-item--danger {
  color: #dc2626;
}

.app-menu-item--danger:hover {
  background: #fef2f2;
}

.app-menu-divider {
  height: 1px;
  background: #f1f5f9;
  margin: 4px 0;
}

.app-menu-has-sub {
  position: relative;
}

.app-menu-has-sub > .app-menu-item {
  justify-content: flex-start;
}

.sub-arrow {
  margin-left: auto;
  font-size: 1.1rem;
  color: #94a3b8;
  line-height: 1;
}

.app-sub-menu {
  display: none;
  position: absolute;
  right: 100%;
  top: -4px;
  min-width: 140px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.15);
  padding: 4px 0;
  z-index: 510;
  margin-right: 2px;
}

.app-menu-has-sub:hover > .app-sub-menu {
  display: block;
}

.app-sub-menu .app-menu-item {
  gap: 6px;
  padding: 8px 14px;
  font-size: 0.84rem;
}

.check-mark {
  width: 16px;
  text-align: center;
  color: var(--qs-primary);
  font-weight: 700;
  font-size: 0.9rem;
  flex-shrink: 0;
}
</style>
