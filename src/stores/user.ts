import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface UserInfo {
  id: number
  username: string
  email?: string
  phone?: string
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

  function setToken(newToken: string) {
    token.value = newToken
    localStorage.setItem('token', newToken)
    isLoggedIn.value = true
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
    localStorage.setItem('userInfo', JSON.stringify(info))
  }

  function logout() {
    token.value = ''
    userInfo.value = null
    isLoggedIn.value = false
    localStorage.removeItem('token')
    localStorage.removeItem('userInfo')
  }

  return { token, userInfo, isLoggedIn, setToken, setUserInfo, logout }
})
