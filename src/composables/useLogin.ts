import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { userLoginApi } from '../api/auth'
import { useUserStore, toFullUrl } from '../stores/user'
import { useCheckin } from './useCheckin'
import { getBrand } from '../brand'
import { getDeviceId } from '../utils/device'
import { getAppCredentials } from '../utils/config'
import { logger } from '../utils/logger'
import { switchToMainLayout, showWindow, ensureLoginSize } from '../utils/window'
import { startHeartbeat } from '../utils/heartbeat'
import { appStorage } from '../utils/storage'

export interface SavedAccount {
  acctno: string
  password: string
  username?: string
  phone?: string
  altAcctno?: string
  avatarPath?: string
}

const ACCOUNTS_KEY = 'saved_accounts'

export function getAccountAvatar(acctno: string): string {
  return appStorage.getItem(`avatar_${acctno}`) || ''
}

function setAccountAvatar(acctno: string, data: string) {
  if (!data) return
  try { appStorage.setItem(`avatar_${acctno}`, data) } catch { /* quota */ }
}

function removeAccountAvatar(acctno: string) {
  appStorage.removeItem(`avatar_${acctno}`)
}

function loadSavedAccounts(): SavedAccount[] {
  try {
    const raw = appStorage.getItem(ACCOUNTS_KEY)
    if (!raw) return []
    const list = JSON.parse(raw) as (SavedAccount & { avatar?: string })[]
    return list.map(({ avatar, ...rest }) => {
      if (avatar) {
        try { appStorage.setItem(`avatar_${rest.acctno}`, avatar) } catch { /* migrate */ }
      }
      return rest
    })
  } catch {
    return []
  }
}

function persistAccounts(accounts: SavedAccount[]) {
  const clean = accounts.map(({ acctno, password, username, phone, altAcctno, avatarPath }) => ({ acctno, password, username, phone, altAcctno, avatarPath }))
  appStorage.setItem(ACCOUNTS_KEY, JSON.stringify(clean))
}

export function getAccountAvatarSrc(acct: SavedAccount): string {
  return getAccountAvatar(acct.acctno) || (acct.avatarPath ? toFullUrl(acct.avatarPath) : '')
}

function isSameUser(saved: SavedAccount, loginAcctno: string, serverPhone?: string, serverAcctno?: string): boolean {
  if (saved.acctno === loginAcctno) return true
  if (serverPhone && (saved.acctno === serverPhone || saved.phone === serverPhone)) return true
  if (serverAcctno && (saved.acctno === serverAcctno || saved.altAcctno === serverAcctno)) return true
  return false
}

export function maskPhone(phone: string) {
  if (phone.length === 11) {
    return phone.slice(0, 3) + '****' + phone.slice(7)
  }
  return phone
}

export function useLogin() {
  const router = useRouter()
  const userStore = useUserStore()

  const cachedAvatar = ref(appStorage.getItem('avatar_data') || '')
  const acctno = ref('')
  const password = ref('')
  const rememberPwd = ref(false)
  const autoLogin = ref(appStorage.getItem('auto_login') === 'true')
  const loading = ref(false)
  const errMsg = ref('')
  const deviceId = ref('')
  const appId = ref('')
  const autoLoginCountdown = ref(0)
  let autoLoginTimer: ReturnType<typeof setInterval> | null = null
  let loginAbortController: AbortController | null = null

  const savedAccounts = ref<SavedAccount[]>([])
  const showDropdown = ref(false)
  const isTyping = ref(false)

  const dropdownAccounts = computed(() => {
    if (!isTyping.value || !acctno.value.trim()) return savedAccounts.value
    const q = acctno.value.trim().toLowerCase()
    return savedAccounts.value.filter(a =>
      a.acctno.includes(q) || (a.username && a.username.toLowerCase().includes(q))
    )
  })

  const shouldShowDropdown = computed(() => {
    return showDropdown.value && savedAccounts.value.length > 0
  })

  watch(autoLogin, (val) => {
    appStorage.setItem('auto_login', String(val))
    if (val) rememberPwd.value = true
  })

  watch(rememberPwd, (val) => {
    if (!val) autoLogin.value = false
  })

  const buttonLabel = computed(() => {
    if (autoLoginCountdown.value > 0) {
      return `自动登录中 (${autoLoginCountdown.value}s) 点击取消`
    }
    if (loading.value) {
      return '登录中… 点击取消'
    }
    return '登 录'
  })

  function startAutoLoginCountdown() {
    autoLoginCountdown.value = 3
    autoLoginTimer = setInterval(() => {
      autoLoginCountdown.value--
      if (autoLoginCountdown.value <= 0) {
        clearAutoLoginTimer()
        handleLogin()
      }
    }, 1000)
  }

  function cancelAutoLogin() {
    clearAutoLoginTimer()
  }

  function clearAutoLoginTimer() {
    if (autoLoginTimer) {
      clearInterval(autoLoginTimer)
      autoLoginTimer = null
    }
    autoLoginCountdown.value = 0
  }

  function saveAccount(acct: SavedAccount, serverPhone?: string, serverAcctno?: string) {
    let list = loadSavedAccounts()
    const dupeIndexes = list.reduce<number[]>((arr, a, i) => {
      if (isSameUser(a, acct.acctno, serverPhone, serverAcctno)) arr.push(i)
      return arr
    }, [])

    if (dupeIndexes.length > 0) {
      for (const idx of dupeIndexes) {
        const old = list[idx]
        if (old.acctno !== acct.acctno) {
          const oldAvatar = getAccountAvatar(old.acctno)
          if (oldAvatar && !getAccountAvatar(acct.acctno)) {
            setAccountAvatar(acct.acctno, oldAvatar)
          }
          removeAccountAvatar(old.acctno)
        }
      }
      list = list.filter((_, i) => !dupeIndexes.includes(i))
    }
    list.unshift(acct)
    persistAccounts(list)
    savedAccounts.value = list
  }

  function removeAccount(acctnoToRemove: string) {
    const list = loadSavedAccounts().filter(a => a.acctno !== acctnoToRemove)
    persistAccounts(list)
    removeAccountAvatar(acctnoToRemove)
    savedAccounts.value = list
  }

  function selectAccount(acct: SavedAccount) {
    acctno.value = acct.acctno
    password.value = acct.password
    rememberPwd.value = true
    showDropdown.value = false
    isTyping.value = false
    errMsg.value = ''
    cachedAvatar.value = getAccountAvatarSrc(acct)
    cancelAutoLogin()
  }

  function handleDeleteAccount(e: Event, acctToDelete: string) {
    e.stopPropagation()
    removeAccount(acctToDelete)
    if (savedAccounts.value.length === 0) {
      showDropdown.value = false
    }
  }

  function onAcctInput() {
    isTyping.value = true
    showDropdown.value = true
    errMsg.value = ''
    password.value = ''
    cancelAutoLogin()
  }

  function onAcctFocus() {
    showDropdown.value = true
  }

  function onAcctBlur() {
    setTimeout(() => {
      showDropdown.value = false
      isTyping.value = false
    }, 200)
  }

  let acctAreaLeaveTimer: ReturnType<typeof setTimeout> | null = null

  function onAcctAreaLeave() {
    acctAreaLeaveTimer = setTimeout(() => {
      showDropdown.value = false
      isTyping.value = false
    }, 300)
  }

  function cancelAcctAreaLeave() {
    if (acctAreaLeaveTimer) {
      clearTimeout(acctAreaLeaveTimer)
      acctAreaLeaveTimer = null
    }
  }

  function toggleDropdown() {
    isTyping.value = false
    showDropdown.value = !showDropdown.value
  }

  function goRegister() {
    router.push('/register')
  }

  function goForgotPassword() {
    router.push('/forgot-password')
  }

  function cancelLogin() {
    if (loginAbortController) {
      loginAbortController.abort()
      loginAbortController = null
    }
    loading.value = false
    errMsg.value = ''
  }

  async function handleLogin() {
    clearAutoLoginTimer()
    errMsg.value = ''
    if (!acctno.value.trim()) {
      errMsg.value = '请输入账号'
      return
    }
    if (!password.value) {
      errMsg.value = '请输入密码'
      return
    }

    loginAbortController = new AbortController()
    loading.value = true
    try {
      const res = await userLoginApi({
        app_id: appId.value,
        acctno: acctno.value.trim(),
        password: password.value,
        device_id: deviceId.value,
        brand_id: getBrand().id,
      }, { signal: loginAbortController.signal })
      if (res.token) {
        userStore.setToken(res.token as string)
      }
      const username = String((res.username ?? res.nickname ?? res.acctno ?? acctno.value.trim()) as string)
      const avatarsPath = typeof res.avatars === 'string' ? res.avatars : undefined
      userStore.setUserInfo({
        id: Number((res.id ?? res.user_id ?? 0) as number | string),
        username,
        email: typeof res.email === 'string' ? res.email : undefined,
        phone: typeof res.phone === 'string' ? res.phone : undefined,
        avatars: avatarsPath,
        acctno: typeof res.acctno === 'string' ? res.acctno : undefined,
        vip_expire_at: typeof res.vip_expire_at === 'string' ? res.vip_expire_at : undefined,
        fen: typeof res.fen === 'number' ? res.fen : undefined,
        app_mode: res.app_mode === 'points' ? 'points' : 'card',
        invite_code: typeof res.invite_code === 'string' ? res.invite_code : undefined,
      })

      if (avatarsPath) {
        await userStore.cacheAvatar(avatarsPath)
      }

      const serverPhone = typeof res.phone === 'string' ? res.phone : undefined
      const serverAcctno = typeof res.acctno === 'string' ? res.acctno : undefined

      if (rememberPwd.value) {
        saveAccount(
          { acctno: acctno.value.trim(), password: password.value, username, phone: serverPhone, altAcctno: serverAcctno, avatarPath: avatarsPath },
          serverPhone,
          serverAcctno,
        )
        const avatarBase64 = appStorage.getItem('avatar_data') || ''
        if (avatarBase64) setAccountAvatar(acctno.value.trim(), avatarBase64)
      } else {
        removeAccount(acctno.value.trim())
      }

      const checkinData = res.checkin as any
      if (checkinData) {
        const { setCheckinInfo } = useCheckin()
        setCheckinInfo(checkinData)
      }

      errMsg.value = ''
      startHeartbeat(userStore.token)
      await switchToMainLayout(router)
    } catch (err: unknown) {
      if (err instanceof DOMException && err.name === 'AbortError') return
      errMsg.value = err instanceof Error ? err.message : '登录失败，请重试'
      logger.error('login', '登录失败', {
        message: err instanceof Error ? err.message : String(err),
      })
    } finally {
      loginAbortController = null
      loading.value = false
    }
  }

  function handleButtonClick() {
    if (autoLoginCountdown.value > 0) {
      cancelAutoLogin()
      return
    }
    if (loading.value) {
      cancelLogin()
      return
    }
    handleLogin()
  }

  function onPasswordInput() {
    errMsg.value = ''
    cancelAutoLogin()
  }

  onMounted(async () => {
    logger.log('login', 'onMounted 开始')
    await ensureLoginSize()

    savedAccounts.value = loadSavedAccounts()
    logger.log('login', '已保存账号列表', savedAccounts.value.map(a => ({
      acctno: a.acctno,
      username: a.username,
      hasAvatar: !!getAccountAvatar(a.acctno),
    })))

    if (savedAccounts.value.length > 0) {
      const last = savedAccounts.value[0]
      acctno.value = last.acctno
      password.value = last.password
      rememberPwd.value = true
      const av = getAccountAvatarSrc(last)
      if (av) cachedAvatar.value = av
    }

    if (!cachedAvatar.value) {
      const userInfoRaw = appStorage.getItem('userInfo')
      if (userInfoRaw) {
        try {
          const info = JSON.parse(userInfoRaw)
          if (info.avatars) {
            cachedAvatar.value = toFullUrl(info.avatars)
            userStore.cacheAvatar(info.avatars)
          }
        } catch { /* ignore */ }
      }
    }

    if (cachedAvatar.value?.startsWith('data:') && acctno.value && !getAccountAvatar(acctno.value)) {
      setAccountAvatar(acctno.value, cachedAvatar.value)
    }

    const [did, creds] = await Promise.all([getDeviceId(), getAppCredentials()])
    deviceId.value = did
    appId.value = creds.appId

    if (userStore.isLoggedIn) {
      logger.log('login', '检测到本地已有登录态，直接切换到主布局')
      loading.value = true
      try {
        await switchToMainLayout(router)
      } catch (e) {
        logger.error('login', '切换到主布局失败', e)
      }
      loading.value = false
      return
    }

    await showWindow()
    logger.log('login', '无登录态，显示登录窗口')

    if (autoLogin.value && acctno.value && password.value) {
      startAutoLoginCountdown()
    }
  })

  onUnmounted(() => {
    clearAutoLoginTimer()
    cancelLogin()
  })

  return {
    router,
    cachedAvatar,
    acctno,
    password,
    rememberPwd,
    autoLogin,
    loading,
    errMsg,
    autoLoginCountdown,
    savedAccounts,
    showDropdown,
    isTyping,
    dropdownAccounts,
    shouldShowDropdown,
    buttonLabel,
    selectAccount,
    handleDeleteAccount,
    onAcctInput,
    onAcctFocus,
    onAcctBlur,
    onAcctAreaLeave,
    cancelAcctAreaLeave,
    toggleDropdown,
    goRegister,
    goForgotPassword,
    handleLogin,
    handleButtonClick,
    onPasswordInput,
  }
}
