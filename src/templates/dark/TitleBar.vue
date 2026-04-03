<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import { invoke } from '@tauri-apps/api/core'
import { showDialog } from '@/utils/dialog'
import { exitApp } from '@/utils/window'
import { getBrand, VERSION } from '../../brand'

defineProps<{
  variant?: 'full' | 'minimal' | 'auth'
  title?: string
}>()

const brand = getBrand()
const APP_BRAND = brand.brand_name
const APP_PRODUCT = brand.product_name
const APP_VERSION = VERSION

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

const expandedSub = ref('')

function closeMenu() {
  showMenu.value = false
  expandedSub.value = ''
}

function toggleMenu(e: MouseEvent) {
  e.stopPropagation()
  showMenu.value = !showMenu.value
  expandedSub.value = ''
}

function toggleSub(name: string) {
  expandedSub.value = expandedSub.value === name ? '' : name
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

async function syncTrayChecks() {
  try {
    await invoke('sync_tray_checks', {
      autostart: autoStartEnabled.value,
      closeMode: closeMode.value,
    })
  } catch { /* ignore */ }
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
    await syncTrayChecks()
  } catch { /* ignore */ }
}

function setCloseMode(mode: 'exit' | 'minimize') {
  showMenu.value = false
  closeMode.value = mode
  localStorage.setItem('close_mode', mode)
  syncTrayChecks()
}

async function onAbout() {
  showMenu.value = false
  await showDialog({
    title: '关于',
    message: `${APP_BRAND}·${APP_PRODUCT} ${APP_VERSION}\n${brand.about}`,
  })
}

async function onExit() {
  showMenu.value = false
  await exitApp()
}
</script>

<template>
  <header class="app-titlebar" :class="{ 'titlebar-compact': variant === 'auth' }" style="-webkit-app-region: drag">
    <div class="tb-left">
      <img class="tb-brand-icon" :src="'/' + brand.logo" :alt="APP_BRAND" />
      <span class="tb-name">{{ APP_BRAND }}</span>
      <span class="tb-dot">·</span>
      <span class="tb-product">{{ APP_PRODUCT }}</span>
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
            <button class="app-menu-item" @click.stop="toggleSub('autostart')">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v4"/><path d="M12 18v4"/><path d="M4.93 4.93l2.83 2.83"/><path d="M16.24 16.24l2.83 2.83"/><path d="M2 12h4"/><path d="M18 12h4"/><path d="M4.93 19.07l2.83-2.83"/><path d="M16.24 7.76l2.83-2.83"/></svg>
              开机自启
              <span class="sub-arrow" :class="{ 'sub-arrow--open': expandedSub === 'autostart' }">›</span>
            </button>
            <div v-if="expandedSub === 'autostart'" class="app-sub-inline">
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
            <button class="app-menu-item" @click.stop="toggleSub('closemode')">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
              关闭设置
              <span class="sub-arrow" :class="{ 'sub-arrow--open': expandedSub === 'closemode' }">›</span>
            </button>
            <div v-if="expandedSub === 'closemode'" class="app-sub-inline">
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
  background: #0F172A;
  border-bottom: 1px solid #1E293B;
  color: #E2E8F0;
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
  width: 26px;
  height: 26px;
  border-radius: 6px;
  object-fit: contain;
  flex-shrink: 0;
}

.tb-name {
  font-weight: 700;
  color: #22D3EE;
  font-size: 0.95rem;
  line-height: 1;
  text-shadow: 0 0 12px rgba(34, 211, 238, 0.4);
}

.tb-dot {
  color: #475569;
  font-size: 0.85rem;
  margin: 0 -4px;
}

.tb-product {
  color: #94A3B8;
  font-size: 0.82rem;
  font-weight: 500;
}

.tb-version {
  color: #22D3EE;
  font-size: 0.65rem;
  font-weight: 600;
  padding: 1px 6px;
  background: rgba(34, 211, 238, 0.1);
  border: 1px solid rgba(34, 211, 238, 0.3);
  border-radius: 4px;
  line-height: 1.4;
}

.tb-sep {
  color: #334155;
  font-size: 0.85rem;
}

.tb-page {
  color: #E2E8F0;
  font-size: 0.88rem;
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
  color: #94A3B8;
  background: #1E293B;
  border: 1px solid #334155;
  cursor: pointer;
  transition: all 0.15s;
}

.win-btn:hover {
  background: #334155;
  color: #E2E8F0;
}

.win-btn--close:hover {
  background: #ef4444;
  color: white;
  border-color: #ef4444;
}

.titlebar-compact {
  height: 36px;
  padding: 0 8px;
  border-bottom: none;
}

.titlebar-compact .tb-brand-icon {
  width: 24px;
  height: 24px;
  font-size: 0.78rem;
  border-radius: 6px;
}

.titlebar-compact .tb-left {
  gap: 6px;
}

.titlebar-compact .tb-name {
  font-size: 0.82rem;
}

.titlebar-compact .tb-product {
  font-size: 0.72rem;
}

.titlebar-compact .tb-version {
  font-size: 0.6rem;
  padding: 0 5px;
}

.titlebar-compact .win-btn {
  width: 30px;
  height: 24px;
  font-size: 13px;
  border-radius: 6px;
}

.titlebar-compact .tb-right {
  gap: 4px;
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
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 10px;
  box-shadow: 0 10px 36px rgba(0, 0, 0, 0.5);
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
  color: #E2E8F0;
  font-size: 0.86rem;
  cursor: pointer;
  transition: background 0.12s;
  white-space: nowrap;
}

.app-menu-item:hover {
  background: #334155;
}

.app-menu-item--danger {
  color: #f87171;
}

.app-menu-item--danger:hover {
  background: rgba(239, 68, 68, 0.15);
}

.app-menu-divider {
  height: 1px;
  background: #334155;
  margin: 4px 0;
}

.app-menu-has-sub > .app-menu-item {
  justify-content: flex-start;
}

.sub-arrow {
  margin-left: auto;
  font-size: 1.1rem;
  color: #64748B;
  line-height: 1;
  transition: transform 0.2s;
}

.sub-arrow--open {
  transform: rotate(90deg);
}

.app-sub-inline {
  background: #0F172A;
  border-top: 1px solid #334155;
}

.app-sub-inline .app-menu-item {
  gap: 6px;
  padding: 7px 16px 7px 40px;
  font-size: 0.82rem;
}

.check-mark {
  width: 16px;
  text-align: center;
  color: #22D3EE;
  font-weight: 700;
  font-size: 0.9rem;
  flex-shrink: 0;
}
</style>
