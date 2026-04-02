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

let unlisten: UnlistenFn | null = null

async function syncTray() {
  try {
    const autostart = await isEnabled()
    const closeMode = localStorage.getItem('close_mode') || 'exit'
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
        localStorage.setItem('close_mode', 'exit')
        break
      case 'close_minimize':
        localStorage.setItem('close_mode', 'minimize')
        break
      case 'about':
        await showDialog({
          title: '关于',
          message: '青拾·视频下载 V1.1.1\n© 2024-2026 青拾',
        })
        break
    }
  })
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>
