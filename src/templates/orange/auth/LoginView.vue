<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import Checkbox from 'primevue/checkbox'
import TitleBar from '../TitleBar.vue'
import { userLoginApi } from '../../../api/auth'
import { useUserStore } from '../../../stores/user'
import { getDeviceId } from '../../../utils/device'
import { getAppCredentials } from '../../../utils/config'
import { logger } from '../../../utils/logger'
import { switchToMainLayout, showWindow, ensureLoginSize } from '../../../utils/window'
import { startHeartbeat } from '../../../utils/heartbeat'
import { getBrand, VERSION } from '../../../brand'

const brand = getBrand()

interface SavedAccount {
  acctno: string
  password: string
  username?: string
  phone?: string
  altAcctno?: string
}

const ACCOUNTS_KEY = 'saved_accounts'

function getAccountAvatar(acctno: string): string {
  return localStorage.getItem(`avatar_${acctno}`) || ''
}

function setAccountAvatar(acctno: string, data: string) {
  if (!data) return
  try { localStorage.setItem(`avatar_${acctno}`, data) } catch { /* quota */ }
}

function removeAccountAvatar(acctno: string) {
  localStorage.removeItem(`avatar_${acctno}`)
}

function loadSavedAccounts(): SavedAccount[] {
  try {
    const raw = localStorage.getItem(ACCOUNTS_KEY)
    if (!raw) return []
    const list = JSON.parse(raw) as (SavedAccount & { avatar?: string })[]
    return list.map(({ avatar, ...rest }) => {
      if (avatar) {
        try { localStorage.setItem(`avatar_${rest.acctno}`, avatar) } catch { /* migrate */ }
      }
      return rest
    })
  } catch {
    return []
  }
}

function persistAccounts(accounts: SavedAccount[]) {
  const clean = accounts.map(({ acctno, password, username, phone, altAcctno }) => ({ acctno, password, username, phone, altAcctno }))
  localStorage.setItem(ACCOUNTS_KEY, JSON.stringify(clean))
}

function isSameUser(saved: SavedAccount, loginAcctno: string, serverPhone?: string, serverAcctno?: string): boolean {
  if (saved.acctno === loginAcctno) return true
  if (serverPhone && (saved.acctno === serverPhone || saved.phone === serverPhone)) return true
  if (serverAcctno && (saved.acctno === serverAcctno || saved.altAcctno === serverAcctno)) return true
  return false
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

const router = useRouter()
const userStore = useUserStore()

const cachedAvatar = ref(localStorage.getItem('avatar_data') || '')

const acctno = ref('')
const password = ref('')
const rememberPwd = ref(false)
const autoLogin = ref(localStorage.getItem('auto_login') === 'true')
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
  localStorage.setItem('auto_login', String(val))
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

function selectAccount(acct: SavedAccount) {
  acctno.value = acct.acctno
  password.value = acct.password
  rememberPwd.value = true
  showDropdown.value = false
  isTyping.value = false
  errMsg.value = ''
  cachedAvatar.value = getAccountAvatar(acct.acctno)
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

function maskPhone(phone: string) {
  if (phone.length === 11) {
    return phone.slice(0, 3) + '****' + phone.slice(7)
  }
  return phone
}

function goRegister() {
  router.push('/register')
}

function goForgotPassword() {
  router.push('/forgot-password')
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
    const av = getAccountAvatar(last.acctno)
    if (av) cachedAvatar.value = av
  }

  if (!cachedAvatar.value) {
    const userInfoRaw = localStorage.getItem('userInfo')
    if (userInfoRaw) {
      try {
        const info = JSON.parse(userInfoRaw)
        if (info.avatars) {
          await userStore.cacheAvatar(info.avatars)
          cachedAvatar.value = localStorage.getItem('avatar_data') || ''
        }
      } catch { /* ignore */ }
    }
  }

  if (cachedAvatar.value && acctno.value && !getAccountAvatar(acctno.value)) {
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
    })

    if (avatarsPath) {
      await userStore.cacheAvatar(avatarsPath)
    }

    const serverPhone = typeof res.phone === 'string' ? res.phone : undefined
    const serverAcctno = typeof res.acctno === 'string' ? res.acctno : undefined

    if (rememberPwd.value) {
      saveAccount(
        { acctno: acctno.value.trim(), password: password.value, username, phone: serverPhone, altAcctno: serverAcctno },
        serverPhone,
        serverAcctno,
      )
      const avatarBase64 = localStorage.getItem('avatar_data') || ''
      if (avatarBase64) setAccountAvatar(acctno.value.trim(), avatarBase64)
    } else {
      removeAccount(acctno.value.trim())
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

onUnmounted(() => {
  clearAutoLoginTimer()
  cancelLogin()
})
</script>

<template>
  <div class="window-shell">
    <TitleBar variant="auth" />
    <div class="deco-arc"></div>

    <div class="body">
      <div class="avatar-wrap">
        <div class="avatar-ring">
          <img v-if="cachedAvatar" :src="cachedAvatar" class="avatar-img" alt="头像" />
          <i v-else class="pi pi-user avatar-placeholder"></i>
        </div>
        <div class="brand-name">{{ brand.brand_name }}</div>
        <div class="product-name">{{ brand.product_name }} {{ VERSION }}</div>
      </div>

      <form class="form" @submit.prevent="handleLogin">
        <div class="field acct-field" @mouseleave="onAcctAreaLeave" @mouseenter="cancelAcctAreaLeave">
          <div class="input-wrap">
            <input
              v-model="acctno"
              type="text"
              placeholder="请输入账号（手机号）"
              class="line-input"
              autocomplete="off"
              @focus="onAcctFocus"
              @blur="onAcctBlur"
              @input="onAcctInput"
            />
            <button
              v-if="savedAccounts.length > 0"
              type="button"
              class="dropdown-btn"
              tabindex="-1"
              @mousedown.prevent="toggleDropdown"
            >
              <svg :class="['arrow-icon', { flip: shouldShowDropdown }]" viewBox="0 0 12 12"><path d="M2.5 4.5L6 8L9.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>
            </button>
          </div>

          <Transition name="dd">
            <div v-if="shouldShowDropdown" class="acct-dropdown">
              <div
                v-for="acct in dropdownAccounts"
                :key="acct.acctno"
                class="dd-item"
                @mousedown.prevent="selectAccount(acct)"
              >
                <div class="dd-avatar">
                  <img v-if="getAccountAvatar(acct.acctno)" :src="getAccountAvatar(acct.acctno)" class="dd-avatar-img" alt="" />
                  <span v-else>{{ (acct.username || acct.acctno).charAt(0) }}</span>
                </div>
                <div class="dd-info">
                  <div class="dd-name">{{ acct.username || maskPhone(acct.acctno) }}</div>
                  <div class="dd-phone">{{ maskPhone(acct.phone || acct.acctno) }}</div>
                </div>
                <button
                  type="button"
                  class="dd-del"
                  title="删除"
                  @mousedown.prevent.stop="handleDeleteAccount($event, acct.acctno)"
                >×</button>
              </div>
              <div v-if="dropdownAccounts.length === 0" class="dd-empty">无匹配账号</div>
            </div>
          </Transition>
        </div>

        <div class="field pw-field">
          <input
            v-model="password"
            type="password"
            placeholder="请输入密码"
            class="line-input"
            autocomplete="current-password"
            @input="onPasswordInput"
          />
          <span
            class="pw-toggle"
            @click="(e: any) => {
              const p = e.target.closest('.pw-field');
              const i = p.querySelector('input');
              i.type = i.type === 'password' ? 'text' : 'password';
              p.classList.toggle('pw-visible');
            }"
          ></span>
        </div>

        <Transition name="fade">
          <div v-if="errMsg" class="err-tip">{{ errMsg }}</div>
        </Transition>

        <div class="options-row">
          <label class="check-label">
            <input type="checkbox" v-model="rememberPwd" class="check-box" />
            <span>记住密码</span>
          </label>
          <label class="check-label">
            <input type="checkbox" v-model="autoLogin" class="check-box" />
            <span>自动登录</span>
          </label>
          <a href="#" class="forgot-link" @click.prevent="goForgotPassword">找回密码</a>
        </div>

        <button type="button" class="submit-btn" @click="handleButtonClick">
          <span v-if="loading && autoLoginCountdown === 0" class="spinner"></span>
          {{ buttonLabel }}
        </button>
      </form>

      <div class="bottom-links">
        <a href="#" @click.prevent="goRegister">注册账号</a>
        <span class="sep">|</span>
        <a href="#" @click.prevent="router.push('/recharge')">卡密充值</a>
        <span class="sep">|</span>
        <a href="#" @click.prevent="router.push('/unbind-device')">解绑设备</a>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in { from { opacity: 0 } to { opacity: 1 } }
@keyframes spin { to { transform: rotate(360deg) } }

.window-shell {
  height: 100vh;
  width: 100vw;
  background: #fff;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  font-family: "HarmonyOS Sans", "PingFang SC", system-ui, sans-serif;
  animation: page-fade-in 0.3s ease-out;
}

.deco-arc {
  position: absolute;
  top: -100px;
  right: -80px;
  width: 260px;
  height: 260px;
  border-radius: 50%;
  background: linear-gradient(135deg, rgba(249, 115, 22, 0.08), rgba(249, 115, 22, 0.02));
  pointer-events: none;
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 32px 44px 16px;
  position: relative;
  z-index: 1;
  min-height: 0;
}

.avatar-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 36px;
  flex-shrink: 0;
}

.avatar-ring {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  border: 2px solid #F97316;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: #fff;
  flex-shrink: 0;
}

.avatar-img { width: 100%; height: 100%; object-fit: cover; }

.avatar-placeholder { font-size: 1.8rem; color: #F97316; }

.brand-name {
  margin-top: 12px;
  font-size: 1.2rem;
  font-weight: 700;
  color: #F97316;
  letter-spacing: 0.04em;
}

.product-name {
  margin-top: 2px;
  font-size: 0.78rem;
  color: #94A3B8;
}

.form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.field { position: relative; width: 100%; }
.input-wrap { position: relative; }

.line-input {
  width: 100%;
  height: 44px;
  border: none;
  border-bottom: 2px solid #E2E8F0;
  background: #fff;
  font-size: 0.92rem;
  color: #1E293B;
  outline: none;
  padding: 0;
  font-family: inherit;
  transition: border-color 0.2s;
  box-sizing: border-box;
}

.line-input:focus { border-bottom-color: #F97316; }
.line-input::placeholder { color: #CBD5E1; }

.acct-field .line-input { padding-right: 34px; }

.dropdown-btn {
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 30px;
  height: 30px;
  border: none;
  background: none;
  color: #94A3B8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: color 0.15s;
}

.dropdown-btn:hover { color: #F97316; }

.arrow-icon {
  width: 14px;
  height: 14px;
  transition: transform 0.2s;
}

.arrow-icon.flip { transform: rotate(180deg); }

.acct-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  background: #fff;
  border: 1px solid #E2E8F0;
  border-radius: 6px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
  z-index: 100;
  max-height: 180px;
  overflow-y: auto;
  padding: 4px;
}

.dd-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.12s;
}

.dd-item:hover { background: #F8FAFC; }

.dd-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #FFF7ED;
  color: #F97316;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.82rem;
  font-weight: 600;
  flex-shrink: 0;
  overflow: hidden;
}

.dd-avatar-img { width: 100%; height: 100%; object-fit: cover; }

.dd-info { flex: 1; min-width: 0; }

.dd-name {
  font-size: 0.85rem;
  font-weight: 500;
  color: #1E293B;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dd-phone { font-size: 0.72rem; color: #94A3B8; }

.dd-del {
  width: 22px;
  height: 22px;
  border: none;
  background: none;
  color: #CBD5E1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-size: 1rem;
  line-height: 1;
  flex-shrink: 0;
  transition: all 0.15s;
  padding: 0;
}

.dd-del:hover { background: #FEF2F2; color: #EF4444; }

.dd-empty {
  padding: 12px;
  text-align: center;
  color: #94A3B8;
  font-size: 0.82rem;
}

.dd-enter-active, .dd-leave-active { transition: all 0.2s ease; }
.dd-enter-from, .dd-leave-to { opacity: 0; transform: translateY(-6px); }

.pw-field .line-input { padding-right: 44px; }

.pw-toggle {
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.8rem;
  color: #94A3B8;
  cursor: pointer;
  user-select: none;
  transition: color 0.15s;
  padding: 4px 0;
}

.pw-toggle::after { content: '显示'; }
.pw-visible .pw-toggle::after { content: '隐藏'; }
.pw-toggle:hover { color: #F97316; }

.err-tip {
  font-size: 0.82rem;
  color: #EF4444;
  margin: -6px 0;
}

.fade-enter-active, .fade-leave-active { transition: all 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.options-row {
  display: flex;
  align-items: center;
  gap: 18px;
}

.check-label {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.82rem;
  color: #64748B;
  cursor: pointer;
  user-select: none;
}

.check-box {
  width: 15px;
  height: 15px;
  accent-color: #F97316;
  cursor: pointer;
  margin: 0;
}

.forgot-link {
  margin-left: auto;
  font-size: 0.82rem;
  color: #94A3B8;
  text-decoration: none;
  transition: color 0.15s;
}

.forgot-link:hover { color: #F97316; }

.submit-btn {
  width: 100%;
  height: 44px;
  background: #F97316;
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 0.95rem;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
}

.submit-btn:hover {
  background: #EA580C;
  transform: translateY(-1px);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.bottom-links {
  margin-top: auto;
  padding: 16px 0 4px;
  text-align: center;
  flex-shrink: 0;
}

.bottom-links a {
  font-size: 0.82rem;
  color: #94A3B8;
  text-decoration: none;
  padding: 4px 8px;
  transition: color 0.15s;
}

.bottom-links a:hover { color: #F97316; }

.sep { color: #E2E8F0; font-size: 0.8rem; }
</style>
