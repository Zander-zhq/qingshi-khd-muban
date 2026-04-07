<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import TitleBar from '../TitleBar.vue'
import { userLoginApi } from '../../../api/auth'
import { useUserStore } from '../../../stores/user'
import { getDeviceId } from '../../../utils/device'
import { getAppCredentials } from '../../../utils/config'
import { logger } from '../../../utils/logger'
import { switchToMainLayout, showWindow, ensureLoginSize } from '../../../utils/window'
import { startHeartbeat } from '../../../utils/heartbeat'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'
import { appStorage } from '../../../utils/storage'
import { toFullUrl } from '../../../stores/user'
import { useCheckin } from '../../../composables/useCheckin'

const brand = getBrand()
const brandLogo = getBrandLogo()

interface SavedAccount {
  acctno: string
  password: string
  username?: string
  phone?: string
  altAcctno?: string
  avatarPath?: string
}

const ACCOUNTS_KEY = 'saved_accounts'

function getAccountAvatar(acctno: string): string {
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

function getAccountAvatarSrc(acct: SavedAccount): string {
  return getAccountAvatar(acct.acctno) || (acct.avatarPath ? toFullUrl(acct.avatarPath) : '')
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
  if (autoLoginCountdown.value > 0) return `自动登录中 (${autoLoginCountdown.value}s) 点击取消`
  if (loading.value) return '登录中… 点击取消'
  return '登 录'
})

function startAutoLoginCountdown() {
  autoLoginCountdown.value = 3
  autoLoginTimer = setInterval(() => {
    autoLoginCountdown.value--
    if (autoLoginCountdown.value <= 0) { clearAutoLoginTimer(); handleLogin() }
  }, 1000)
}

function cancelAutoLogin() { clearAutoLoginTimer() }

function clearAutoLoginTimer() {
  if (autoLoginTimer) { clearInterval(autoLoginTimer); autoLoginTimer = null }
  autoLoginCountdown.value = 0
}

function selectAccount(acct: SavedAccount) {
  acctno.value = acct.acctno; password.value = acct.password; rememberPwd.value = true
  showDropdown.value = false; isTyping.value = false; errMsg.value = ''
  cachedAvatar.value = getAccountAvatarSrc(acct); cancelAutoLogin()
}

function handleDeleteAccount(e: Event, acctToDelete: string) {
  e.stopPropagation(); removeAccount(acctToDelete)
  if (savedAccounts.value.length === 0) showDropdown.value = false
}

function onAcctInput() { isTyping.value = true; showDropdown.value = true; errMsg.value = ''; password.value = ''; cancelAutoLogin() }
function onAcctFocus() { showDropdown.value = true }
function onAcctBlur() { setTimeout(() => { showDropdown.value = false; isTyping.value = false }, 200) }

let acctAreaLeaveTimer: ReturnType<typeof setTimeout> | null = null
function onAcctAreaLeave() { acctAreaLeaveTimer = setTimeout(() => { showDropdown.value = false; isTyping.value = false }, 300) }
function cancelAcctAreaLeave() { if (acctAreaLeaveTimer) { clearTimeout(acctAreaLeaveTimer); acctAreaLeaveTimer = null } }
function toggleDropdown() { isTyping.value = false; showDropdown.value = !showDropdown.value }

function maskPhone(phone: string) { return phone.length === 11 ? phone.slice(0, 3) + '****' + phone.slice(7) : phone }

function goRegister() { router.push('/register') }
function goForgotPassword() { router.push('/forgot-password') }

onMounted(async () => {
  logger.log('login', 'onMounted 开始')
  await ensureLoginSize()

  savedAccounts.value = loadSavedAccounts()
  logger.log('login', '已保存账号列表', savedAccounts.value.map(a => ({
    acctno: a.acctno, username: a.username, hasAvatar: !!getAccountAvatar(a.acctno),
  })))

  if (savedAccounts.value.length > 0) {
    const last = savedAccounts.value[0]
    acctno.value = last.acctno; password.value = last.password; rememberPwd.value = true
    const av = getAccountAvatarSrc(last)
    if (av) cachedAvatar.value = av
  }

  if (!cachedAvatar.value) {
    const userInfoRaw = appStorage.getItem('userInfo')
    if (userInfoRaw) {
      try {
        const info = JSON.parse(userInfoRaw)
        if (info.avatars) { cachedAvatar.value = toFullUrl(info.avatars); userStore.cacheAvatar(info.avatars) }
      } catch { /* ignore */ }
    }
  }

  if (cachedAvatar.value?.startsWith('data:') && acctno.value && !getAccountAvatar(acctno.value)) {
    setAccountAvatar(acctno.value, cachedAvatar.value)
  }

  const [did, creds] = await Promise.all([getDeviceId(), getAppCredentials()])
  deviceId.value = did; appId.value = creds.appId

  if (userStore.isLoggedIn) {
    logger.log('login', '检测到本地已有登录态，直接切换到主布局')
    loading.value = true
    try { await switchToMainLayout(router) } catch (e) { logger.error('login', '切换到主布局失败', e) }
    loading.value = false; return
  }

  await showWindow()
  logger.log('login', '无登录态，显示登录窗口')

  if (autoLogin.value && acctno.value && password.value) startAutoLoginCountdown()
})

function cancelLogin() {
  if (loginAbortController) { loginAbortController.abort(); loginAbortController = null }
  loading.value = false; errMsg.value = ''
}

async function handleLogin() {
  clearAutoLoginTimer(); errMsg.value = ''
  if (!acctno.value.trim()) { errMsg.value = '请输入账号'; return }
  if (!password.value) { errMsg.value = '请输入密码'; return }

  loginAbortController = new AbortController(); loading.value = true
  try {
    const res = await userLoginApi({ app_id: appId.value, acctno: acctno.value.trim(), password: password.value, device_id: deviceId.value, brand_id: getBrand().id }, { signal: loginAbortController.signal })
    if (res.token) userStore.setToken(res.token as string)
    const username = String((res.username ?? res.nickname ?? res.acctno ?? acctno.value.trim()) as string)
    const avatarsPath = typeof res.avatars === 'string' ? res.avatars : undefined
    userStore.setUserInfo({
      id: Number((res.id ?? res.user_id ?? 0) as number | string), username,
      email: typeof res.email === 'string' ? res.email : undefined,
      phone: typeof res.phone === 'string' ? res.phone : undefined,
      avatars: avatarsPath, acctno: typeof res.acctno === 'string' ? res.acctno : undefined,
      vip_expire_at: typeof res.vip_expire_at === 'string' ? res.vip_expire_at : undefined,
      fen: typeof res.fen === 'number' ? res.fen : undefined,
      app_mode: res.app_mode === 'points' ? 'points' : 'card',
      invite_code: typeof res.invite_code === 'string' ? res.invite_code : undefined,
    })
    if (avatarsPath) await userStore.cacheAvatar(avatarsPath)

    const serverPhone = typeof res.phone === 'string' ? res.phone : undefined
    const serverAcctno = typeof res.acctno === 'string' ? res.acctno : undefined

    if (rememberPwd.value) {
      saveAccount({ acctno: acctno.value.trim(), password: password.value, username, phone: serverPhone, altAcctno: serverAcctno, avatarPath: avatarsPath }, serverPhone, serverAcctno)
      const avatarBase64 = appStorage.getItem('avatar_data') || ''
      if (avatarBase64) setAccountAvatar(acctno.value.trim(), avatarBase64)
    } else { removeAccount(acctno.value.trim()) }

    const checkinData = res.checkin as any
    if (checkinData) { const { setCheckinInfo } = useCheckin(); setCheckinInfo(checkinData) }

    errMsg.value = ''; startHeartbeat(userStore.token); await switchToMainLayout(router)
  } catch (err: unknown) {
    if (err instanceof DOMException && err.name === 'AbortError') return
    errMsg.value = err instanceof Error ? err.message : '登录失败，请重试'
    logger.error('login', '登录失败', { message: err instanceof Error ? err.message : String(err) })
  } finally { loginAbortController = null; loading.value = false }
}

function handleButtonClick() {
  if (autoLoginCountdown.value > 0) { cancelAutoLogin(); return }
  if (loading.value) { cancelLogin(); return }
  handleLogin()
}

function onPasswordInput() { errMsg.value = ''; cancelAutoLogin() }

onUnmounted(() => { clearAutoLoginTimer(); cancelLogin() })
</script>

<template>
  <div class="login-shell">
    <TitleBar variant="auth" />

    <div class="login-split">
      <!-- ========== Left: Brand Panel ========== -->
      <div class="left-panel">
        <div class="lp-deco lp-deco-1"></div>
        <div class="lp-deco lp-deco-2"></div>
        <div class="lp-deco lp-deco-3"></div>

        <div class="lp-content">
          <div class="lp-logo">
            <img :src="brandLogo" alt="" class="lp-logo-img" />
          </div>
          <h1 class="lp-title">{{ brand.brand_name }}</h1>
          <p class="lp-subtitle">{{ brand.product_name }}</p>
          <div class="lp-divider"></div>
          <ul class="lp-features">
            <li><i class="pi pi-check-circle"></i> 高速稳定下载</li>
            <li><i class="pi pi-check-circle"></i> 多平台支持</li>
            <li><i class="pi pi-check-circle"></i> 安全可靠</li>
          </ul>
        </div>

        <div class="lp-version">{{ VERSION }}</div>
      </div>

      <!-- ========== Right: Login Form ========== -->
      <div class="right-panel">
        <div class="rp-content">
          <div class="rp-header">
            <div class="rp-avatar">
              <img v-if="cachedAvatar" :src="cachedAvatar" class="rp-avatar-img" alt="" />
              <i v-else class="pi pi-user rp-avatar-icon"></i>
            </div>
            <h2 class="rp-title">用户登录</h2>
            <p class="rp-desc">请输入您的账号信息</p>
          </div>

          <form class="rp-form" @submit.prevent="handleLogin">
            <!-- Account -->
            <div class="field acct-field" @mouseleave="onAcctAreaLeave" @mouseenter="cancelAcctAreaLeave">
              <div class="input-box">
                <i class="pi pi-user input-icon"></i>
                <input v-model="acctno" type="text" placeholder="账号 / 手机号" class="gk-input" autocomplete="off" @focus="onAcctFocus" @blur="onAcctBlur" @input="onAcctInput" />
                <button v-if="savedAccounts.length > 0" type="button" class="dd-trigger" tabindex="-1" @mousedown.prevent="toggleDropdown">
                  <svg :class="['caret', { flip: shouldShowDropdown }]" viewBox="0 0 12 12"><path d="M2.5 4.5L6 8L9.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>
                </button>
              </div>
              <Transition name="dd">
                <div v-if="shouldShowDropdown" class="acct-dropdown">
                  <div v-for="acct in dropdownAccounts" :key="acct.acctno" class="dd-row" @mousedown.prevent="selectAccount(acct)">
                    <div class="dd-av"><img v-if="getAccountAvatarSrc(acct)" :src="getAccountAvatarSrc(acct)" class="dd-av-img" alt="" /><span v-else>{{ (acct.username || acct.acctno).charAt(0) }}</span></div>
                    <div class="dd-info"><div class="dd-name">{{ acct.username || maskPhone(acct.acctno) }}</div><div class="dd-sub">{{ maskPhone(acct.phone || acct.acctno) }}</div></div>
                    <button type="button" class="dd-del" title="删除" @mousedown.prevent.stop="handleDeleteAccount($event, acct.acctno)">×</button>
                  </div>
                  <div v-if="dropdownAccounts.length === 0" class="dd-empty">无匹配账号</div>
                </div>
              </Transition>
            </div>

            <!-- Password -->
            <div class="field pw-field">
              <div class="input-box">
                <i class="pi pi-lock input-icon"></i>
                <input v-model="password" type="password" placeholder="密码" class="gk-input" autocomplete="current-password" @input="onPasswordInput" />
                <span class="pw-eye" @click="(e: any) => { const box = e.target.closest('.input-box'); const inp = box.querySelector('input'); inp.type = inp.type === 'password' ? 'text' : 'password'; box.classList.toggle('pw-visible'); }"></span>
              </div>
            </div>

            <!-- Error -->
            <Transition name="fade">
              <div v-if="errMsg" class="err-msg"><i class="pi pi-exclamation-circle"></i> {{ errMsg }}</div>
            </Transition>

            <!-- Options -->
            <div class="opts">
              <label class="opt"><input type="checkbox" v-model="rememberPwd" class="opt-cb" /><span>记住密码</span></label>
              <label class="opt"><input type="checkbox" v-model="autoLogin" class="opt-cb" /><span>自动登录</span></label>
              <a href="#" class="opt-link" @click.prevent="goForgotPassword">忘记密码？</a>
            </div>

            <!-- Login button -->
            <button type="button" class="login-btn" @click="handleButtonClick">
              <span v-if="loading && autoLoginCountdown === 0" class="spinner"></span>
              {{ buttonLabel }}
            </button>
          </form>

          <!-- Links -->
          <div class="rp-links">
            <a href="#" @click.prevent="goRegister"><i class="pi pi-user-plus"></i> 注册账号</a>
            <a href="#" @click.prevent="router.push('/recharge')"><i class="pi pi-credit-card"></i> 卡密充值</a>
            <a href="#" @click.prevent="router.push('/unbind-device')"><i class="pi pi-link"></i> 解绑设备</a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes fade-in { from { opacity: 0 } to { opacity: 1 } }
@keyframes spin { to { transform: rotate(360deg) } }

.login-shell {
  height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow-y: auto;
  font-family: "HarmonyOS Sans", "PingFang SC", "Helvetica Neue", system-ui, sans-serif;
  animation: fade-in 0.3s ease-out;
}

.login-split { flex: 1; display: flex; min-height: 0; }

/* ═══ Left Panel ═══ */
.left-panel {
  width: 260px; flex-shrink: 0;
  background: linear-gradient(160deg, #F97316 0%, #EA580C 50%, #C2410C 100%);
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  position: relative; overflow: hidden; color: #fff;
}

.lp-deco { position: absolute; border-radius: 50%; background: rgba(255,255,255,0.08); pointer-events: none; }
.lp-deco-1 { width: 200px; height: 200px; top: -60px; left: -40px; }
.lp-deco-2 { width: 140px; height: 140px; bottom: -30px; right: -30px; }
.lp-deco-3 { width: 80px; height: 80px; top: 50%; right: 20px; background: rgba(255,255,255,0.04); }

.lp-content { position: relative; z-index: 1; text-align: center; padding: 0 30px; }

.lp-logo { margin-bottom: 16px; }
.lp-logo-img { width: 56px; height: 56px; border-radius: 12px; border: 2px solid rgba(255,255,255,0.3); object-fit: contain; }

.lp-title { margin: 0; font-size: 1.6rem; font-weight: 800; letter-spacing: 0.06em; }
.lp-subtitle { margin: 4px 0 0; font-size: 0.88rem; opacity: 0.8; font-weight: 400; }

.lp-divider { width: 40px; height: 3px; background: rgba(255,255,255,0.4); border-radius: 2px; margin: 20px auto; }

.lp-features { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 10px; text-align: left; }
.lp-features li { display: flex; align-items: center; gap: 8px; font-size: 0.82rem; opacity: 0.85; }
.lp-features i { font-size: 0.78rem; }

.lp-version {
  position: absolute; bottom: 16px; left: 0; right: 0; text-align: center;
  font-size: 0.65rem; opacity: 0.5; letter-spacing: 0.08em;
}

/* ═══ Right Panel ═══ */
.right-panel {
  flex: 1; background: #fff; display: flex; align-items: center; justify-content: center;
}

.rp-content { width: 100%; max-width: 340px; padding: 0 32px; }

.rp-header { text-align: center; margin-bottom: 24px; }

.rp-avatar {
  width: 52px; height: 52px; border-radius: 50%; margin: 0 auto 10px;
  border: 2px solid #F97316; display: flex; align-items: center; justify-content: center;
  overflow: hidden; background: #FFF7ED;
}

.rp-avatar-img { width: 100%; height: 100%; object-fit: cover; }
.rp-avatar-icon { font-size: 1.3rem; color: #F97316; }

.rp-title { margin: 0; font-size: 1.15rem; font-weight: 700; color: #303133; }
.rp-desc { margin: 4px 0 0; font-size: 0.78rem; color: #909399; }

/* ═══ Form ═══ */
.rp-form { display: flex; flex-direction: column; gap: 14px; }

.field { position: relative; width: 100%; }
.input-box { position: relative; display: flex; align-items: center; }
.input-icon { position: absolute; left: 12px; font-size: 0.85rem; color: #909399; pointer-events: none; z-index: 1; }

.gk-input {
  width: 100%; height: 40px; border: 1px solid #DCDFE6; border-radius: 4px; background: #fff;
  font-size: 0.85rem; color: #303133; padding: 0 36px; outline: none; font-family: inherit;
  transition: border-color 0.2s, box-shadow 0.2s; box-sizing: border-box;
}

.gk-input:focus { border-color: #F97316; box-shadow: 0 0 0 2px rgba(249,115,22,0.1); }
.gk-input::placeholder { color: #C0C4CC; }

.dd-trigger {
  position: absolute; right: 4px; top: 50%; transform: translateY(-50%);
  width: 26px; height: 26px; border: none; background: none; color: #909399;
  cursor: pointer; display: flex; align-items: center; justify-content: center; border-radius: 4px;
}
.dd-trigger:hover { color: #F97316; background: #FFF7ED; }
.caret { width: 12px; height: 12px; transition: transform 0.2s; }
.caret.flip { transform: rotate(180deg); }

.acct-dropdown {
  position: absolute; top: calc(100% + 4px); left: 0; right: 0; background: #fff;
  border: 1px solid #EBEEF5; border-radius: 4px; box-shadow: 0 2px 12px rgba(0,0,0,0.1);
  z-index: 100; max-height: 160px; overflow-y: auto; padding: 4px;
}

.dd-row { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 4px; cursor: pointer; transition: background 0.12s; }
.dd-row:hover { background: #F5F7FA; }
.dd-av { width: 28px; height: 28px; border-radius: 50%; background: #FFF7ED; color: #F97316; display: flex; align-items: center; justify-content: center; font-size: 0.72rem; font-weight: 600; flex-shrink: 0; overflow: hidden; }
.dd-av-img { width: 100%; height: 100%; object-fit: cover; }
.dd-info { flex: 1; min-width: 0; }
.dd-name { font-size: 0.78rem; font-weight: 500; color: #303133; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.dd-sub { font-size: 0.65rem; color: #909399; }
.dd-del { width: 18px; height: 18px; border: none; background: none; color: #C0C4CC; cursor: pointer; display: flex; align-items: center; justify-content: center; border-radius: 4px; font-size: 0.9rem; flex-shrink: 0; padding: 0; }
.dd-del:hover { background: #FEF0F0; color: #F56C6C; }
.dd-empty { padding: 10px; text-align: center; color: #909399; font-size: 0.75rem; }
.dd-enter-active, .dd-leave-active { transition: all 0.2s ease; }
.dd-enter-from, .dd-leave-to { opacity: 0; transform: translateY(-4px); }

.pw-field .gk-input { padding-right: 44px; }
.pw-eye { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); font-size: 0.75rem; color: #909399; cursor: pointer; user-select: none; }
.pw-eye::after { content: '显示'; }
.pw-visible .pw-eye::after { content: '隐藏'; }
.pw-eye:hover { color: #F97316; }

.err-msg { display: flex; align-items: center; gap: 6px; font-size: 0.75rem; color: #F56C6C; padding: 6px 10px; background: #FEF0F0; border: 1px solid #FDE2E2; border-radius: 4px; margin: -4px 0; }
.fade-enter-active, .fade-leave-active { transition: all 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.opts { display: flex; align-items: center; gap: 14px; }
.opt { display: flex; align-items: center; gap: 4px; font-size: 0.75rem; color: #606266; cursor: pointer; user-select: none; }
.opt-cb { width: 14px; height: 14px; accent-color: #F97316; cursor: pointer; margin: 0; }
.opt-link { margin-left: auto; font-size: 0.75rem; color: #909399; text-decoration: none; }
.opt-link:hover { color: #F97316; }

.login-btn {
  width: 100%; height: 40px; background: #F97316; color: #fff; border: none; border-radius: 4px;
  font-size: 0.88rem; font-weight: 600; cursor: pointer; font-family: inherit;
  display: flex; align-items: center; justify-content: center; gap: 6px; transition: background 0.2s;
}

.login-btn:hover { background: #EA580C; }
.login-btn:active { background: #C2410C; }

.spinner { width: 14px; height: 14px; border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff; border-radius: 50%; animation: spin 0.6s linear infinite; }

/* ═══ Bottom Links ═══ */
.rp-links {
  display: flex; justify-content: center; gap: 20px; margin-top: 20px; padding-top: 16px;
  border-top: 1px solid #EBEEF5;
}

.rp-links a {
  font-size: 0.75rem; color: #909399; text-decoration: none;
  display: inline-flex; align-items: center; gap: 4px; transition: color 0.15s;
}

.rp-links a i { font-size: 0.72rem; }
.rp-links a:hover { color: #F97316; }
</style>
