<template>
  <router-view />
  <GlobalDialog />
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { showDialog } from '@/utils/dialog'
import { syncServerTime } from '@/utils/sign'
import { fetchAppConfig } from '@/utils/config'
import { getBrand, VERSION } from '@/brand'
import { appStorage } from '@/utils/storage'

let unlisten: UnlistenFn | null = null

async function syncTray() {
  try {
    const autostart = await isEnabled()
    const closeMode = appStorage.getItem('close_mode') || 'exit'
    await invoke('sync_tray_checks', { autostart, closeMode })
  } catch { /* ignore */ }
}

onMounted(async () => {
  await syncServerTime()
  await fetchAppConfig()
  await syncTray()

  unlisten = await listen<string>('tray-action', async (event) => {
    const action = event.payload
    switch (action) {
      case 'refresh':
        window.location.reload()
        break
      case 'clear_cache': {
        const keepKeys = ['saved_accounts', 'token', 'userInfo', 'close_mode', 'brand_config', 'active_brand_id']
        const saved: Record<string, string> = {}
        keepKeys.forEach(k => {
          const v = appStorage.getItem(k)
          if (v) saved[k] = v
        })
        sessionStorage.clear()
        appStorage.clear()
        Object.entries(saved).forEach(([k, v]) => appStorage.setItem(k, v))
        window.location.reload()
        break
      }
      case 'autostart_on':
        try { await enable() } catch { /* ignore */ }
        await syncTray()
        break
      case 'autostart_off':
        try { await disable() } catch { /* ignore */ }
        await syncTray()
        break
      case 'close_exit':
        appStorage.setItem('close_mode', 'exit')
        break
      case 'close_minimize':
        appStorage.setItem('close_mode', 'minimize')
        break
      case 'about': {
        const brand = getBrand()
        await showDialog({
          title: '关于',
          message: `${brand.brand_name}·${brand.product_name} ${VERSION}\n${brand.about || `© ${new Date().getFullYear()} ${brand.brand_name}`}`,
        })
        break
      }
    }
  })
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>
