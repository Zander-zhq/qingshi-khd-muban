import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getAppCredentials } from '../utils/config'
import { checkUpdate } from '../api/version'
import type { CheckUpdateResult } from '../api/version'
import { VERSION } from '../brand'
import { logger } from '../utils/logger'
import { appStorage } from '../utils/storage'
import { stopHeartbeat, callLogoutApi } from '../utils/heartbeat'


const SKIP_KEY = 'skip_version'

export function useVersionUpdate() {
  const showUpdateDialog = ref(false)
  const updateInfo = ref<CheckUpdateResult | null>(null)
  const downloadProgress = ref(0)
  const downloadStatus = ref<'idle' | 'downloading' | 'verifying' | 'installing' | 'done' | 'error'>('idle')
  const downloadError = ref('')
  const selectedUpdateIdx = ref(0)

  async function checkForUpdate(manual = false): Promise<boolean> {
    try {
      const { appId } = await getAppCredentials()
      const currentVersion = VERSION.replace(/^[vV]/, '')
      logger.log('version', '检查更新', { appId, currentVersion })
      const res = await checkUpdate(appId, currentVersion)
      logger.log('version', '检查更新结果', res)

      if (!res.has_update) {
        if (manual) updateInfo.value = res
        return false
      }

      const skipped = appStorage.getItem(SKIP_KEY)
      if (!manual && !res.force_update && skipped === res.latest_version) {
        return false
      }

      updateInfo.value = res
      selectedUpdateIdx.value = 0
      showUpdateDialog.value = true
      return true
    } catch (err) {
      logger.error('version', '检查更新失败', err)
      return false
    }
  }

  async function applyUpdate() {
    if (!updateInfo.value?.download_url) return
    downloadStatus.value = 'downloading'
    downloadProgress.value = 0
    downloadError.value = ''

    const unlisten = await listen<{ loaded: number; total: number | null; percent: number }>('download_file_progress', (event) => {
      downloadProgress.value = Math.round(event.payload.percent)
    })

    try {
      const saveDir = await invoke<string>('get_download_dir').catch(() => '.')
      const installerPath = await invoke<string>('download_file_to_dir', {
        url: updateInfo.value.download_url,
        saveDir,
      })
      logger.log('version', '下载完成', { installerPath })

      const expectedHash = updateInfo.value.file_hash
      if (expectedHash) {
        downloadStatus.value = 'verifying'
        const actualHash = await invoke<string>('compute_file_sha256', { path: installerPath })
        if (actualHash.toLowerCase() !== expectedHash.toLowerCase()) {
          throw new Error('安装包校验失败，文件可能已被篡改，请重试')
        }
        logger.log('version', 'SHA256 校验通过')
      }

      downloadStatus.value = 'installing'

      try {
        stopHeartbeat()
        const { useUserStore } = await import('../stores/user')
        const userStore = useUserStore()
        if (userStore.token) {
          await callLogoutApi(userStore.token)
          logger.log('version', '更新前退出登录成功')
        }
      } catch (e) {
        logger.warn('version', '更新前退出登录失败，继续安装', e)
      }

      await invoke('run_installer_and_exit', { installerPath })
    } catch (err) {
      downloadStatus.value = 'error'
      downloadError.value = err instanceof Error ? err.message : String(err)
      logger.error('version', '更新失败', err)
    } finally {
      unlisten()
    }
  }

  function dismissUpdate() {
    if (updateInfo.value?.force_update) return
    if (updateInfo.value?.latest_version) {
      appStorage.setItem(SKIP_KEY, updateInfo.value.latest_version)
    }
    showUpdateDialog.value = false
  }

  return {
    showUpdateDialog,
    updateInfo,
    downloadProgress,
    downloadStatus,
    downloadError,
    selectedUpdateIdx,
    checkForUpdate,
    applyUpdate,
    dismissUpdate,
  }
}
