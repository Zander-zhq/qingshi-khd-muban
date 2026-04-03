<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import Checkbox from 'primevue/checkbox'
import TitleBar from '../../components/TitleBar.vue'
import { userLoginApi } from '../../api/auth'
import { useUserStore } from '../../stores/user'
import { getDeviceId } from '../../utils/device'
import { getAppCredentials } from '../../utils/config'
import { logger } from '../../utils/logger'
import { switchToMainLayout, showWindow, ensureLoginSize } from '../../utils/window'
import { startHeartbeat } from '../../utils/heartbeat'

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
    hasAvatar: !!a.avatar,
    avatarLen: a.avatar ? a.avatar.length : 0,
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
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="bc bc-3"></div>
        <div class="banner-title">登录</div>
      </div>

      <div class="body">
        <div class="avatar-ring">
          <div class="avatar">
            <img v-if="cachedAvatar" :src="cachedAvatar" class="avatar-img" alt="头像" />
            <i v-else class="pi pi-user"></i>
          </div>
        </div>

        <form class="form" @submit.prevent="handleLogin">
          <div class="field-box acct-field" @mouseleave="onAcctAreaLeave" @mouseenter="cancelAcctAreaLeave">
            <div class="acct-input-wrap">
              <InputText
                v-model="acctno"
                placeholder="请输入账号（手机号）"
                class="field-input"
                autocomplete="off"
                @focus="onAcctFocus"
                @blur="onAcctBlur"
                @input="onAcctInput"
              />
              <button
                v-if="savedAccounts.length > 0"
                type="button"
                class="acct-dropdown-toggle"
                tabindex="-1"
                @mousedown.prevent="toggleDropdown"
              >
                <i class="pi" :class="shouldShowDropdown ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
              </button>
            </div>

            <Transition name="dropdown">
              <div v-if="shouldShowDropdown" class="acct-dropdown">
                <div
                  v-for="acct in dropdownAccounts"
                  :key="acct.acctno"
                  class="acct-item"
                  @mousedown.prevent="selectAccount(acct)"
                >
                  <div class="acct-item-avatar">
                    <img v-if="getAccountAvatar(acct.acctno)" :src="getAccountAvatar(acct.acctno)" class="acct-avatar-img" alt="" />
                    <span v-else>{{ (acct.username || acct.acctno).charAt(0) }}</span>
                  </div>
                  <div class="acct-item-info">
                    <div class="acct-item-name">{{ acct.username || maskPhone(acct.acctno) }}</div>
                    <div class="acct-item-phone">{{ maskPhone(acct.phone || acct.acctno) }}</div>
                  </div>
                  <button
                    type="button"
                    class="acct-item-del"
                    title="删除此账号"
                    @mousedown.prevent.stop="handleDeleteAccount($event, acct.acctno)"
                  >
                    <i class="pi pi-times"></i>
                  </button>
                </div>
                <div v-if="dropdownAccounts.length === 0" class="acct-empty">无匹配账号</div>
              </div>
            </Transition>
          </div>

          <div class="field-box">
            <Password
              v-model="password"
              placeholder="请输入密码"
              :feedback="false"
              toggleMask
              class="field-pw"
              inputClass="field-input"
              autocomplete="current-password"
              @input="onPasswordInput"
            />
          </div>

          <Transition name="fade">
            <div v-if="errMsg" class="err-tip">
              <i class="pi pi-exclamation-circle"></i>
              {{ errMsg }}
            </div>
          </Transition>

          <div class="options-row">
            <div class="option-checks">
              <div class="check-item">
                <Checkbox v-model="rememberPwd" :binary="true" inputId="rememberPwd" />
                <label for="rememberPwd">记住密码</label>
              </div>
              <div class="check-item">
                <Checkbox v-model="autoLogin" :binary="true" inputId="autoLogin" />
                <label for="autoLogin">自动登录</label>
              </div>
            </div>
            <a href="#" class="link-text" @click.prevent="goForgotPassword">找回密码</a>
          </div>

          <Button
            type="button"
            :label="buttonLabel"
            :icon="loading && autoLoginCountdown === 0 ? 'pi pi-spin pi-spinner' : undefined"
            class="submit-btn"
            @click="handleButtonClick"
          />
        </form>

        <div class="bottom-links">
          <a class="bottom-link" href="#" @click.prevent="goRegister">注册账号</a>
          <span class="bottom-sep">|</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/recharge')">卡密充值</a>
          <span class="bottom-sep">|</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/unbind-device')">解绑设备</a>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.window-shell {
  height: 100vh;
  width: 100vw;
  background: #fff;
  animation: page-fade-in 0.35s ease-out;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
  overflow: hidden;
  position: relative;
}

.banner {
  height: 140px;
  position: relative;
  background: var(--qs-bg-gradient);
  flex-shrink: 0;
  overflow: visible;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bc {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.07);
  pointer-events: none;
}
.bc-1 { width: 200px; height: 200px; top: -100px; right: -40px; }
.bc-2 { width: 120px; height: 120px; bottom: -60px; left: 20px; }
.bc-3 { width: 70px; height: 70px; top: 10px; left: 38%; background: rgba(255,255,255,0.05); }

.banner-title {
  position: relative;
  z-index: 1;
  font-size: 1.8rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.15em;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0 40px 0;
  min-height: 0;
}

.avatar-ring {
  margin-top: -44px;
  margin-bottom: 14px;
  z-index: 2;
  padding: 4px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 4px 20px rgba(13, 148, 136, 0.12);
  flex-shrink: 0;
}

.avatar {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: linear-gradient(135deg, #ccfbf1, #f0fdfa);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--qs-primary);
  font-size: 2rem;
  overflow: hidden;
}

.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field-box {
  width: 100%;
}

.acct-field {
  position: relative;
}

.acct-input-wrap {
  position: relative;
  width: 100%;
}

.acct-input-wrap :deep(.field-input) {
  padding-right: 38px;
}

.acct-dropdown-toggle {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.15s;
  font-size: 0.75rem;
}

.acct-dropdown-toggle:hover {
  background: #f1f5f9;
  color: #64748b;
}

.acct-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.1);
  z-index: 100;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px;
}

.acct-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s;
}

.acct-item:hover {
  background: #f0fdfa;
}

.acct-item-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.82rem;
  font-weight: 600;
  flex-shrink: 0;
  overflow: hidden;
}

.acct-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.acct-item-info {
  flex: 1;
  min-width: 0;
}

.acct-item-name {
  font-size: 0.88rem;
  font-weight: 500;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.acct-item-phone {
  font-size: 0.75rem;
  color: #94a3b8;
}

.acct-item-del {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: #cbd5e1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  transition: all 0.15s;
  flex-shrink: 0;
  font-size: 0.7rem;
}

.acct-item-del:hover {
  background: #fef2f2;
  color: #ef4444;
}

.acct-empty {
  padding: 12px;
  text-align: center;
  color: #94a3b8;
  font-size: 0.82rem;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%;
  height: 46px;
  font-size: 0.95rem;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  background: #f8fafb;
  padding: 0 16px;
  transition: all 0.2s;
}

:deep(.field-pw) {
  width: 100%;
}

.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: var(--qs-primary-light);
  background: #fff;
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.08);
}

.err-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  color: #e74c3c;
  padding: 8px 12px;
  background: #fef2f2;
  border-radius: 8px;
  border: 1px solid #fecaca;
  margin: -4px 0;
}

.fade-enter-active,
.fade-leave-active {
  transition: all 0.25s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.options-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.option-checks {
  display: flex;
  align-items: center;
  gap: 16px;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.check-item label {
  font-size: 0.85rem;
  color: var(--qs-text-secondary);
  cursor: pointer;
}

.link-text {
  font-size: 0.85rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  transition: color 0.15s;
}

.link-text:hover {
  color: var(--qs-primary-dark);
}

.submit-btn {
  width: 100%;
  height: 46px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 23px;
  margin-top: 4px;
  background: var(--qs-bg-gradient) !important;
  border: none !important;
  box-shadow: 0 4px 16px rgba(13, 148, 136, 0.3);
  transition: all 0.2s;
}

.submit-btn:hover {
  box-shadow: 0 6px 24px rgba(13, 148, 136, 0.4);
  transform: translateY(-1px);
}

.bottom-links {
  margin-top: auto;
  padding: 12px 0 14px;
  text-align: center;
  flex-shrink: 0;
}

.bottom-link {
  display: inline-block;
  padding: 6px 10px;
  font-size: 0.82rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s;
}

.bottom-link:hover {
  color: var(--qs-primary-dark);
}

.bottom-sep {
  color: #cbd5e1;
  font-size: 0.8rem;
}
</style>
