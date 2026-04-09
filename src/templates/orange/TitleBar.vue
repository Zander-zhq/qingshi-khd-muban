<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import { showDialog } from '@/utils/dialog'
import { exitApp } from '@/utils/window'
import { openUrl } from '@tauri-apps/plugin-opener'
import { appStorage } from '../../utils/storage'
import { getBrand, getBrandLogo, VERSION } from '../../brand'

defineProps<{
  variant?: 'full' | 'minimal' | 'auth'
  title?: string
  contactFloatVisible?: boolean
}>()

const emit = defineEmits<{ 'restore-contact': []; 'open-disclaimer': [] }>()

const brand = getBrand()
const brandLogo = getBrandLogo()
const APP_BRAND = brand.brand_name
const APP_PRODUCT = brand.product_name
const APP_VERSION = VERSION

const appWindow = getCurrentWindow()
const isMaximized = ref(false)
const showMenu = ref(false)
const closeMode = ref<'exit' | 'minimize'>((appStorage.getItem('close_mode') as 'exit' | 'minimize') || 'exit')
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

async function handleMinimize() { await appWindow.minimize() }
async function handleToggleMaximize() { await appWindow.toggleMaximize() }
async function handleClose() {
  if (closeMode.value === 'minimize') { await appWindow.hide() } else { await exitApp() }
}

function onRefresh() { showMenu.value = false; window.location.reload() }
function onClearCache() {
  showMenu.value = false
  const keepKeys = ['saved_accounts', 'token', 'userInfo', 'close_mode', 'brand_config', 'active_brand_id']
  const saved: Record<string, string> = {}
  keepKeys.forEach(k => { const v = appStorage.getItem(k); if (v) saved[k] = v })
  sessionStorage.clear(); appStorage.clear()
  Object.entries(saved).forEach(([k, v]) => appStorage.setItem(k, v))
  window.location.reload()
}

async function setAutoStart(enabled: boolean) {
  showMenu.value = false
  try { if (enabled) { await enable() } else { await disable() }; autoStartEnabled.value = await isEnabled() } catch { /* */ }
}

function setCloseMode(mode: 'exit' | 'minimize') {
  showMenu.value = false; closeMode.value = mode; appStorage.setItem('close_mode', mode)
}

function onWebsite() {
  showMenu.value = false
  if (brand.website) openUrl(brand.website)
}

async function onAbout() {
  showMenu.value = false
  await showDialog({ title: '关于', message: `${APP_BRAND}·${APP_PRODUCT} ${APP_VERSION}\n${brand.about}` })
}

async function onExit() { showMenu.value = false; await exitApp() }
</script>

<template>
  <header class="app-titlebar" :class="{ 'titlebar-compact': variant === 'auth' }" style="-webkit-app-region: drag">
    <div class="tb-left">
      <img class="tb-brand-icon" :src="brandLogo" :alt="APP_BRAND" />
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
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><line x1="4" y1="6" x2="20" y2="6"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="18" x2="20" y2="18"/></svg>
        </button>
        <div v-if="showMenu" class="app-menu-dropdown" @click.stop>
          <button class="app-menu-item" @click="onRefresh"><i class="pi pi-refresh"></i> 刷新界面</button>
          <button class="app-menu-item" @click="onClearCache"><i class="pi pi-trash"></i> 清理缓存</button>
          <div class="app-menu-divider" />
          <div class="app-menu-has-sub">
            <button class="app-menu-item" @click.stop="toggleSub('autostart')">开机自启 <span class="sub-arrow" :class="{ 'sub-arrow--open': expandedSub === 'autostart' }">›</span></button>
            <div v-if="expandedSub === 'autostart'" class="app-sub-inline">
              <button class="app-menu-item" @click="setAutoStart(true)"><span class="check-mark">{{ autoStartEnabled ? '✓' : '' }}</span> 开启</button>
              <button class="app-menu-item" @click="setAutoStart(false)"><span class="check-mark">{{ !autoStartEnabled ? '✓' : '' }}</span> 关闭</button>
            </div>
          </div>
          <div class="app-menu-has-sub">
            <button class="app-menu-item" @click.stop="toggleSub('closemode')">关闭设置 <span class="sub-arrow" :class="{ 'sub-arrow--open': expandedSub === 'closemode' }">›</span></button>
            <div v-if="expandedSub === 'closemode'" class="app-sub-inline">
              <button class="app-menu-item" @click="setCloseMode('exit')"><span class="check-mark">{{ closeMode === 'exit' ? '✓' : '' }}</span> 直接退出</button>
              <button class="app-menu-item" @click="setCloseMode('minimize')"><span class="check-mark">{{ closeMode === 'minimize' ? '✓' : '' }}</span> 最小化到后台</button>
            </div>
          </div>
          <div class="app-menu-divider" />
          <button v-if="brand.website" class="app-menu-item" @click="onWebsite"><i class="pi pi-globe"></i> 访问官网</button>
          <button v-if="brand.contact_images?.length && !contactFloatVisible" class="app-menu-item" @click="showMenu = false; emit('restore-contact')"><i class="pi pi-comments"></i> 联系我们</button>
          <button v-if="brand.disclaimer" class="app-menu-item" @click="showMenu = false; emit('open-disclaimer')"><i class="pi pi-shield"></i> 免责声明</button>
          <button v-if="brand.about" class="app-menu-item" @click="onAbout"><i class="pi pi-info-circle"></i> 关于</button>
          <button class="app-menu-item app-menu-item--danger" @click="onExit"><i class="pi pi-sign-out"></i> 退出程序</button>
        </div>
      </div>
      <button class="win-btn" title="最小化" @click="handleMinimize">–</button>
      <button class="win-btn" title="最大化/还原" @click="handleToggleMaximize">{{ isMaximized ? '❐' : '▢' }}</button>
      <button class="win-btn win-btn--close" title="关闭" @click="handleClose">✕</button>
    </div>
  </header>
</template>

<style scoped>
.app-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 10px;
  background: #fff;
  border-bottom: 1px solid #EBEEF5;
  color: #303133;
  font-size: 0.85rem;
  flex-shrink: 0;
  z-index: 200;
}

.tb-left { display: flex; align-items: center; gap: 8px; }
.tb-brand-icon { width: 22px; height: 22px; border-radius: 4px; object-fit: contain; flex-shrink: 0; }
.tb-name { font-weight: 700; color: #F97316; font-size: 0.88rem; }
.tb-dot { color: #DCDFE6; font-size: 0.8rem; margin: 0 -3px; }
.tb-product { color: #606266; font-size: 0.78rem; font-weight: 500; }
.tb-version { color: #F97316; font-size: 0.6rem; font-weight: 600; padding: 1px 5px; background: #FFF7ED; border: 1px solid #FED7AA; border-radius: 3px; }
.tb-sep { color: #DCDFE6; font-size: 0.8rem; }
.tb-page { color: #303133; font-size: 0.82rem; font-weight: 500; }

.tb-right { display: flex; align-items: center; gap: 4px; margin-left: auto; }

.win-btn {
  width: 32px;
  height: 26px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 500;
  color: #606266;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.15s;
}

.win-btn:hover { background: #F5F7FA; color: #303133; }
.win-btn--close:hover { background: #F56C6C; color: #fff; }

.titlebar-compact { height: 32px; padding: 0 8px; }
.titlebar-compact .tb-brand-icon { width: 18px; height: 18px; }
.titlebar-compact .tb-left { gap: 6px; }
.titlebar-compact .tb-name { font-size: 0.8rem; }
.titlebar-compact .tb-product { font-size: 0.7rem; }
.titlebar-compact .tb-version { font-size: 0.55rem; padding: 0 4px; }
.titlebar-compact .win-btn { width: 28px; height: 22px; font-size: 11px; }
.titlebar-compact .tb-right { gap: 2px; }

.app-menu-wrap { position: relative; }
.app-menu-trigger { font-size: 12px; }

.app-menu-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 168px;
  background: #fff;
  border: 1px solid #EBEEF5;
  border-radius: 4px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
  padding: 4px 0;
  z-index: 500;
}

.app-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 16px;
  border: none;
  background: none;
  color: #606266;
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.12s;
  white-space: nowrap;
}

.app-menu-item:hover { background: #F5F7FA; color: #303133; }
.app-menu-item i { font-size: 0.85rem; width: 16px; text-align: center; color: #909399; }
.app-menu-item:hover i { color: #F97316; }
.app-menu-item--danger { color: #F56C6C; }
.app-menu-item--danger i { color: #F56C6C; }
.app-menu-item--danger:hover { background: #FEF0F0; color: #F56C6C; }
.app-menu-divider { height: 1px; background: #EBEEF5; margin: 4px 0; }

.app-menu-has-sub > .app-menu-item { justify-content: flex-start; }
.sub-arrow { margin-left: auto; font-size: 1rem; color: #909399; transition: transform 0.2s; }
.sub-arrow--open { transform: rotate(90deg); }
.app-sub-inline { background: #F5F7FA; border-top: 1px solid #EBEEF5; }
.app-sub-inline .app-menu-item { gap: 4px; padding: 6px 16px 6px 36px; font-size: 0.78rem; }
.check-mark { width: 14px; text-align: center; color: #F97316; font-weight: 700; font-size: 0.85rem; flex-shrink: 0; }
</style>
