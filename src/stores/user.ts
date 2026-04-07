import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { appStorage } from '../utils/storage'
import { logger } from '../utils/logger'

export interface UserInfo {
  id: number
  username: string
  email?: string
  phone?: string
  avatars?: string
  acctno?: string
  vip_expire_at?: string
  fen?: number
  app_mode?: 'card' | 'points'
  invite_code?: string
}

const AVATAR_DATA_KEY = 'avatar_data'
const AVATAR_URL_KEY = 'avatar_url'
const API_BASE = import.meta.env.VITE_API_BASE_URL as string

export function toFullUrl(path: string): string {
  if (!path) return ''
  if (path.startsWith('http://') || path.startsWith('https://') || path.startsWith('blob:') || path.startsWith('data:')) return path
  return API_BASE + path
}

export const useUserStore = defineStore('user', () => {
  let initialUserInfo: UserInfo | null = null
  const cachedUserInfo = appStorage.getItem('userInfo')
  if (cachedUserInfo) {
    try {
      initialUserInfo = JSON.parse(cachedUserInfo) as UserInfo
    } catch {
      appStorage.removeItem('userInfo')
    }
  }

  const token = ref<string>(appStorage.getItem('token') || '')
  const userInfo = ref<UserInfo | null>(initialUserInfo)
  const isLoggedIn = ref(!!token.value)
  const avatarData = ref(appStorage.getItem(AVATAR_DATA_KEY) || '')

  const avatarUrl = computed(() => {
    if (avatarData.value) return avatarData.value
    const raw = userInfo.value?.avatars || ''
    return toFullUrl(raw)
  })

  function resizeImageToBase64(imgSrc: string, maxSize = 128): Promise<string> {
    return new Promise((resolve, reject) => {
      const img = new Image()
      img.crossOrigin = 'anonymous'
      img.onload = () => {
        const w = Math.min(img.naturalWidth, maxSize)
        const h = Math.min(img.naturalHeight, maxSize)
        const size = Math.min(w, h)
        const canvas = document.createElement('canvas')
        canvas.width = size
        canvas.height = size
        const ctx = canvas.getContext('2d')!
        ctx.imageSmoothingQuality = 'high'
        ctx.drawImage(img, 0, 0, size, size)
        resolve(canvas.toDataURL('image/png'))
      }
      img.onerror = () => reject(new Error('Image load failed'))
      img.src = imgSrc
    })
  }

  async function downloadImageAsBase64(url: string): Promise<string> {
    try {
      const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
      const resp = await tauriFetch(url)
      if (resp.ok) {
        const blob = await resp.blob()
        const blobUrl = URL.createObjectURL(blob)
        try {
          return await resizeImageToBase64(blobUrl)
        } finally {
          URL.revokeObjectURL(blobUrl)
        }
      }
    } catch { /* tauri fetch failed, try canvas */ }

    try {
      return await resizeImageToBase64(url)
    } catch { /* canvas fallback also failed */ }

    return ''
  }

  async function cacheAvatar(serverPath: string) {
    if (!serverPath) return
    const cachedUrl = appStorage.getItem(AVATAR_URL_KEY) || ''
    if (cachedUrl === serverPath && avatarData.value) return

    const fullUrl = toFullUrl(serverPath)
    const base64 = await downloadImageAsBase64(fullUrl)
    if (base64) {
      avatarData.value = base64
      try {
        appStorage.setItem(AVATAR_DATA_KEY, base64)
        appStorage.setItem(AVATAR_URL_KEY, serverPath)
      } catch { /* quota exceeded */ }
      syncPerAccountAvatar(base64)
    } else {
      logger.warn('user-store', 'cacheAvatar 缓存失败', { url: fullUrl })
    }
  }

  function syncPerAccountAvatar(base64: string) {
    if (!base64 || !userInfo.value) return
    const keys = [userInfo.value.acctno, userInfo.value.phone].filter(Boolean) as string[]
    for (const key of keys) {
      try { appStorage.setItem(`avatar_${key}`, base64) } catch { /* quota */ }
    }
  }

  function setToken(newToken: string) {
    token.value = newToken
    appStorage.setItem('token', newToken)
    isLoggedIn.value = true
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
    appStorage.setItem('userInfo', JSON.stringify(info))
    if (info.avatars) {
      cacheAvatar(info.avatars)
    } else {
      avatarData.value = ''
      appStorage.removeItem(AVATAR_DATA_KEY)
      appStorage.removeItem(AVATAR_URL_KEY)
    }
  }

  function updateUserInfo(partial: Partial<UserInfo>) {
    if (!userInfo.value) return
    const updated = { ...userInfo.value, ...partial }
    userInfo.value = updated
    appStorage.setItem('userInfo', JSON.stringify(updated))
    if (partial.avatars) {
      cacheAvatar(partial.avatars)
      syncSavedAccountsAvatarPath(partial.avatars)
    }
  }

  function syncSavedAccountsAvatarPath(avatarPath: string) {
    try {
      const raw = appStorage.getItem('saved_accounts')
      if (!raw) return
      const list = JSON.parse(raw) as Record<string, unknown>[]
      const phone = userInfo.value?.phone
      const acctno = userInfo.value?.acctno
      let changed = false
      for (const item of list) {
        if ((phone && item.acctno === phone) || (acctno && (item.acctno === acctno || item.altAcctno === acctno)) || (phone && item.phone === phone)) {
          item.avatarPath = avatarPath
          changed = true
        }
      }
      if (changed) appStorage.setItem('saved_accounts', JSON.stringify(list))
    } catch { /* ignore */ }
  }

  function logout() {
    token.value = ''
    userInfo.value = null
    isLoggedIn.value = false
    appStorage.removeItem('token')
    appStorage.removeItem('userInfo')
  }

  return { token, userInfo, isLoggedIn, avatarUrl, setToken, setUserInfo, updateUserInfo, logout, cacheAvatar }
})
