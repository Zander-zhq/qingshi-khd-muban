import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { fetch } from '@tauri-apps/plugin-http'

export interface UserInfo {
  id: number
  username: string
  email?: string
  phone?: string
  avatars?: string
  acctno?: string
}

const AVATAR_DATA_KEY = 'avatar_data'
const AVATAR_URL_KEY = 'avatar_url'
const API_BASE = import.meta.env.VITE_API_BASE_URL as string

function toFullUrl(path: string): string {
  if (!path) return ''
  if (path.startsWith('http://') || path.startsWith('https://') || path.startsWith('blob:') || path.startsWith('data:')) return path
  return API_BASE + path
}

export const useUserStore = defineStore('user', () => {
  let initialUserInfo: UserInfo | null = null
  const cachedUserInfo = localStorage.getItem('userInfo')
  if (cachedUserInfo) {
    try {
      initialUserInfo = JSON.parse(cachedUserInfo) as UserInfo
    } catch {
      localStorage.removeItem('userInfo')
    }
  }

  const token = ref<string>(localStorage.getItem('token') || '')
  const userInfo = ref<UserInfo | null>(initialUserInfo)
  const isLoggedIn = ref(!!token.value)
  const avatarData = ref(localStorage.getItem(AVATAR_DATA_KEY) || '')

  const avatarUrl = computed(() => {
    if (avatarData.value) return avatarData.value
    const raw = userInfo.value?.avatars || ''
    return toFullUrl(raw)
  })

  async function cacheAvatar(serverPath: string) {
    if (!serverPath) return
    const cachedUrl = localStorage.getItem(AVATAR_URL_KEY) || ''
    if (cachedUrl === serverPath && avatarData.value) return

    try {
      const fullUrl = toFullUrl(serverPath)
      const resp = await fetch(fullUrl)
      if (!resp.ok) return
      const blob = await resp.blob()
      const reader = new FileReader()
      const base64 = await new Promise<string>((resolve) => {
        reader.onloadend = () => resolve(reader.result as string)
        reader.readAsDataURL(blob)
      })
      avatarData.value = base64
      localStorage.setItem(AVATAR_DATA_KEY, base64)
      localStorage.setItem(AVATAR_URL_KEY, serverPath)
    } catch { /* 下载失败不影响使用 */ }
  }

  function setToken(newToken: string) {
    token.value = newToken
    localStorage.setItem('token', newToken)
    isLoggedIn.value = true
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
    localStorage.setItem('userInfo', JSON.stringify(info))
    if (info.avatars) {
      cacheAvatar(info.avatars)
    } else {
      avatarData.value = ''
      localStorage.removeItem(AVATAR_DATA_KEY)
      localStorage.removeItem(AVATAR_URL_KEY)
    }
  }

  function updateUserInfo(partial: Partial<UserInfo>) {
    if (!userInfo.value) return
    const updated = { ...userInfo.value, ...partial }
    userInfo.value = updated
    localStorage.setItem('userInfo', JSON.stringify(updated))
    if (partial.avatars) {
      cacheAvatar(partial.avatars)
    }
  }

  function logout() {
    token.value = ''
    userInfo.value = null
    isLoggedIn.value = false
    localStorage.removeItem('token')
    localStorage.removeItem('userInfo')
  }

  return { token, userInfo, isLoggedIn, avatarUrl, setToken, setUserInfo, updateUserInfo, logout, cacheAvatar }
})
