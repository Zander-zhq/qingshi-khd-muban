<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import Checkbox from 'primevue/checkbox'
import { userLoginApi } from '../../api/auth'
import { useUserStore } from '../../stores/user'
import { getDeviceId } from '../../utils/device'
import { getAppCredentials } from '../../utils/config'
import { logger } from '../../utils/logger'
import { activateMainWindow, exitApp } from '../../utils/window'

interface SavedAccount {
  acctno: string
  password: string
  username?: string
}

const ACCOUNTS_KEY = 'saved_accounts'

function loadSavedAccounts(): SavedAccount[] {
  try {
    const raw = localStorage.getItem(ACCOUNTS_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

function persistAccounts(accounts: SavedAccount[]) {
  localStorage.setItem(ACCOUNTS_KEY, JSON.stringify(accounts))
}

function saveAccount(acct: SavedAccount) {
  const list = loadSavedAccounts()
  const idx = list.findIndex(a => a.acctno === acct.acctno)
  if (idx >= 0) {
    list[idx] = acct
  } else {
    list.unshift(acct)
  }
  persistAccounts(list)
  savedAccounts.value = list
}

function removeAccount(acctno: string) {
  const list = loadSavedAccounts().filter(a => a.acctno !== acctno)
  persistAccounts(list)
  savedAccounts.value = list
}

const router = useRouter()
const userStore = useUserStore()
const appWindow = getCurrentWindow()

async function handleMinimize() {
  await appWindow.minimize()
}

async function handleCloseWindow() {
  await exitApp()
}

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
  savedAccounts.value = loadSavedAccounts()

  if (savedAccounts.value.length > 0) {
    const last = savedAccounts.value[0]
    acctno.value = last.acctno
    password.value = last.password
    rememberPwd.value = true
  }

  const [did, creds] = await Promise.all([getDeviceId(), getAppCredentials()])
  deviceId.value = did
  appId.value = creds.appId

  await appWindow.show()
  await appWindow.setFocus()

  if (userStore.isLoggedIn) {
    logger.log('login', '检测到本地已有登录态，直接激活主窗口')
    loading.value = true
    await new Promise(resolve => setTimeout(resolve, 1500))
    await activateMainWindow()
    loading.value = false
    return
  }

  if (autoLogin.value && acctno.value && password.value) {
    startAutoLoginCountdown()
  }
})

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

  loading.value = true
  try {
    const res = await userLoginApi({
      app_id: appId.value,
      acctno: acctno.value.trim(),
      password: password.value,
      device_id: deviceId.value,
    })
    if (res.token) {
      userStore.setToken(res.token as string)
    }
    const username = String((res.username ?? res.nickname ?? res.acctno ?? acctno.value.trim()) as string)
    userStore.setUserInfo({
      id: Number((res.id ?? res.user_id ?? 0) as number | string),
      username,
      email: typeof res.email === 'string' ? res.email : undefined,
      phone: typeof res.phone === 'string' ? res.phone : undefined,
    })

    if (rememberPwd.value) {
      saveAccount({ acctno: acctno.value.trim(), password: password.value, username })
    } else {
      removeAccount(acctno.value.trim())
    }

    errMsg.value = ''
    await activateMainWindow()
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '登录失败，请重试'
    logger.error('login', '登录失败', {
      message: err instanceof Error ? err.message : String(err),
    })
  } finally {
    loading.value = false
  }
}

function handleButtonClick() {
  if (autoLoginCountdown.value > 0) {
    cancelAutoLogin()
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
})
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <div class="banner" style="-webkit-app-region: drag">
        <div class="login-window-actions" style="-webkit-app-region: no-drag">
          <button class="login-win-btn" @click="handleMinimize" title="最小化">
            <svg width="12" height="12" viewBox="0 0 12 12"><rect y="5" width="12" height="1.5" rx="0.75" fill="currentColor"/></svg>
          </button>
          <button class="login-win-btn login-win-btn--close" @click="handleCloseWindow" title="关闭">
            <svg width="12" height="12" viewBox="0 0 12 12">
              <path d="M1.5 1.5L10.5 10.5M10.5 1.5L1.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="bc bc-3"></div>
        <div class="logo-text">青拾</div>
      </div>

      <div class="body">
        <div class="avatar-ring">
          <div class="avatar">
            <i class="pi pi-user"></i>
          </div>
        </div>

        <form class="form" @submit.prevent="handleLogin">
          <div class="field-box acct-field">
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
                  <div class="acct-item-avatar">{{ (acct.username || acct.acctno).charAt(0) }}</div>
                  <div class="acct-item-info">
                    <div class="acct-item-name">{{ acct.username || maskPhone(acct.acctno) }}</div>
                    <div class="acct-item-phone">{{ maskPhone(acct.acctno) }}</div>
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
            :loading="loading && autoLoginCountdown === 0"
            class="submit-btn"
            @click="handleButtonClick"
          />
        </form>

        <div class="bottom-links">
          <a class="bottom-link" href="#" @click.prevent="goRegister">注册账号</a>
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
  height: 150px;
  position: relative;
  background: var(--qs-bg-gradient);
  flex-shrink: 0;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  padding-top: 10px;
}

.login-window-actions {
  position: absolute;
  top: 6px;
  right: 6px;
  display: flex;
  align-items: center;
  gap: 2px;
  z-index: 10;
}

.login-win-btn {
  width: 30px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.75);
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.15s;
}

.login-win-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.login-win-btn--close:hover {
  background: #ef4444;
  color: #fff;
}

.bc {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.07);
}
.bc-1 { width: 200px; height: 200px; top: -70px; right: -40px; }
.bc-2 { width: 120px; height: 120px; bottom: -40px; left: 20px; }
.bc-3 { width: 70px; height: 70px; top: 25px; left: 38%; background: rgba(255,255,255,0.05); }

.logo-text {
  position: relative;
  z-index: 1;
  font-size: 2.6rem;
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
  padding: 0 40px 24px;
}

.avatar-ring {
  margin-top: -34px;
  margin-bottom: 24px;
  z-index: 2;
  padding: 3px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 4px 20px rgba(13, 148, 136, 0.12);
}

.avatar {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: linear-gradient(135deg, #ccfbf1, #f0fdfa);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--qs-primary);
  font-size: 1.6rem;
}

.form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 16px;
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
  padding: 16px 0 12px;
  text-align: center;
}

.bottom-link {
  display: inline-block;
  padding: 6px 16px;
  font-size: 0.85rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s;
}

.bottom-link:hover {
  color: var(--qs-primary-dark);
}
</style>
