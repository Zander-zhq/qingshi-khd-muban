<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from './TitleBar.vue'
import { useUserStore } from '../../stores/user'
import { useCheckin } from '../../composables/useCheckin'
import { logger } from '../../utils/logger'
import { switchToLoginLayout } from '../../utils/window'
import { startHeartbeat, stopHeartbeat, callLogoutApi, setHeartbeatCallbacks } from '../../utils/heartbeat'
import { showDialog } from '../../utils/dialog'
import { updateProfileApi, sendEmailCodeApi, bindEmailApi, changePasswordApi, redeemCardInnerApi, unbindDeviceInnerApi } from '../../api/auth'
import { uploadImage } from '../../api/brand'
import { getBrand } from '../../brand'
import appRoutes from '../../app/routes'
import { fetchPackages, createOrder, queryOrder } from '../../api/pay'
import type { PayPackage } from '../../api/pay'
import QRCode from 'qrcode'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useContact } from '../../composables/useContact'
import { useVersionUpdate } from '../../composables/useVersionUpdate'
import { useDisclaimer } from '../../composables/useDisclaimer'
import DisclaimerDialog from '../../components/DisclaimerDialog.vue'
import { getAppCredentials, getUnbindTip } from '../../utils/config'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const brand = getBrand()

const {
  hasContactImages, showContactFloat, showContactModal, contactHighlight, cachedImages,
  closeContactFloat, restoreContactFloat,
} = useContact()

const {
  showUpdateDialog, updateInfo, downloadProgress, downloadStatus, downloadError,
  selectedUpdateIdx, checkForUpdate, applyUpdate, dismissUpdate,
} = useVersionUpdate()

const {
  checkinLoading, canCheckin, rewardSummary, showCheckinHint, doCheckin, clearCheckin,
} = useCheckin()

const {
  showDisclaimerModal, openDisclaimer, closeDisclaimer,
  acceptDisclaimer, checkAndShowOnFirstLogin,
} = useDisclaimer()

const checkinMsg = ref('')

async function handleCheckin() {
  try {
    const res = await doCheckin()
    if (res) {
      checkinMsg.value = res.reward_summary ? `签到成功！获得 ${res.reward_summary}` : '签到成功！'
      setTimeout(() => checkinMsg.value = '', 3000)
    }
  } catch (err: unknown) {
    checkinMsg.value = err instanceof Error ? err.message : '签到失败'
    setTimeout(() => checkinMsg.value = '', 3000)
  }
}

const displayName = computed(() => userStore.userInfo?.username || '用户')
const appMenuItems = appRoutes
  .filter(r => r.meta?.menuItem)
  .sort((a, b) => ((a.meta!.menuItem as any).order ?? 99) - ((b.meta!.menuItem as any).order ?? 99))
  .map(r => {
    const mi = r.meta!.menuItem as { label: string; icon: string }
    return { label: mi.label, icon: mi.icon, path: `/main/${String(r.path)}` }
  })

const menuItems = [
  { label: '仪表盘', icon: 'pi pi-home', path: '/main/dashboard' },
  ...appMenuItems,
  ...(import.meta.env.DEV ? [
    { label: '品牌管理', icon: 'pi pi-palette', path: '/main/dev-brand' },
    { label: '版本管理', icon: 'pi pi-tag', path: '/main/dev-version' },
  ] : []),
]

const pageTitle = computed(() => {
  const found = menuItems.find(m => route.path === m.path)
  if (found) return found.label
  return '主页'
})

const collapsed = ref(false)
const showUserMenu = ref(false)
const showBanned = ref(false)
const bannedTitle = ref('账号已被禁用')
const bannedMsg = ref('')
const bannedCountdown = ref(5)
const showExpired = ref(false)
const expiredMsg = ref('')
const forceExpired = ref(false)

function toggleSidebar() {
  collapsed.value = !collapsed.value
}

function toggleUserMenu(e: MouseEvent) {
  e.stopPropagation()
  showUserMenu.value = !showUserMenu.value
}

function closeUserMenu() {
  showUserMenu.value = false
}

onMounted(() => {
  if (userStore.token) {
    startHeartbeat(userStore.token)
  }
  if (!import.meta.env.DEV) checkForUpdate()
  document.addEventListener('click', closeUserMenu)

  function countdownLogout(seconds: number) {
    bannedCountdown.value = seconds
    const t = setInterval(async () => {
      bannedCountdown.value--
      if (bannedCountdown.value <= 0) {
        clearInterval(t)
        userStore.logout()
        await switchToLoginLayout(router)
      }
    }, 1000)
  }

  setHeartbeatCallbacks({
    onBanned(msg) {
      bannedTitle.value = '账号已被禁用'
      bannedMsg.value = msg
      showBanned.value = true
      countdownLogout(5)
    },
    onExpired(msg) {
      expiredMsg.value = msg
      showExpired.value = true
      forceExpired.value = true
    },
    async onSessionExpired(msg) {
      logger.warn('main-layout', '会话过期，即将跳转登录', { msg })
      await showDialog({ title: '登录已过期', message: msg || '您的登录已过期，请重新登录' })
      userStore.logout()
      await switchToLoginLayout(router)
    },
    async onDeviceKicked(msg) {
      bannedTitle.value = '设备已被下线'
      bannedMsg.value = msg
      showBanned.value = true
      countdownLogout(5)
    },
    async onDeviceMismatch(msg) {
      await showDialog({ title: '设备异常', message: msg })
      userStore.logout()
      await switchToLoginLayout(router)
    },
    async onServerUnreachable(msg) {
      logger.warn('main-layout', '服务器不可达，强制下线', { msg })
      await showDialog({ title: '连接断开', message: msg || '无法连接到服务器，请检查网络后重新登录' })
      userStore.logout()
      await switchToLoginLayout(router)
    },
  })

  const info = userStore.userInfo
  if (info && (!info.acctno || !info.email)) {
    setTimeout(() => handleEditProfile(), 500)
  }

  setTimeout(() => checkAndShowOnFirstLogin(), 800)
})

onUnmounted(() => {
  stopHeartbeat()
  document.removeEventListener('click', closeUserMenu)
})

async function handleNavigate(path: string) {
  if (path === '#disclaimer') {
    openDisclaimer()
    return
  }
  logger.log('main-layout', '点击菜单导航', { path })
  await router.push(path)
}

async function handleLogout() {
  showUserMenu.value = false
  logger.log('main-layout', '用户退出登录')
  stopHeartbeat()
  if (userStore.token) {
    await callLogoutApi(userStore.token)
  }
  clearCheckin()
  userStore.logout()
  await switchToLoginLayout(router)
}

const showProfileModal = ref(false)
const profileNickname = ref('')
const profileAcctno = ref('')
const profileEmail = ref('')
const profileEmailCode = ref('')
const emailEditing = ref(false)
const emailSending = ref(false)
const emailCooldown = ref(0)
let emailCooldownTimer: ReturnType<typeof setInterval> | null = null
const emailBtnText = computed(() => emailCooldown.value > 0 ? `${emailCooldown.value}s` : '发送验证码')
const profileAvatarPreview = ref('')
const profileAvatarFile = ref<File | null>(null)
const profileLoading = ref(false)
const profileErrMsg = ref('')
const profileSuccessMsg = ref('')
const avatarInputRef = ref<HTMLInputElement | null>(null)

function startEmailCooldown() {
  emailCooldown.value = 60
  emailCooldownTimer = setInterval(() => {
    emailCooldown.value--
    if (emailCooldown.value <= 0 && emailCooldownTimer) {
      clearInterval(emailCooldownTimer)
      emailCooldownTimer = null
    }
  }, 1000)
}

async function handleSendEmailCode() {
  profileErrMsg.value = ''
  if (!profileEmail.value || !profileEmail.value.includes('@')) {
    profileErrMsg.value = '请输入正确的邮箱地址'
    return
  }
  emailSending.value = true
  try {
    const { appId } = await getAppCredentials()
    await sendEmailCodeApi({ app_id: appId, email: profileEmail.value.trim(), scene: 'bind_email' })
    profileSuccessMsg.value = '验证码已发送到邮箱'
    startEmailCooldown()
  } catch (err: unknown) {
    profileErrMsg.value = err instanceof Error ? err.message : '发送失败'
  } finally {
    emailSending.value = false
  }
}

function handleEditProfile() {
  showUserMenu.value = false
  profileNickname.value = userStore.userInfo?.username || ''
  profileAcctno.value = userStore.userInfo?.acctno || ''
  profileEmail.value = userStore.userInfo?.email || ''
  profileEmailCode.value = ''
  emailEditing.value = !userStore.userInfo?.email
  profileAvatarPreview.value = userStore.avatarUrl
  profileAvatarFile.value = null
  profileErrMsg.value = ''
  profileSuccessMsg.value = ''
  showProfileModal.value = true
}

function closeProfileModal() {
  showProfileModal.value = false
  if (profileAvatarPreview.value?.startsWith('blob:')) {
    URL.revokeObjectURL(profileAvatarPreview.value)
  }
  if (emailCooldownTimer) { clearInterval(emailCooldownTimer); emailCooldownTimer = null }
}

function triggerAvatarSelect() {
  avatarInputRef.value?.click()
}

function onAvatarSelected(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  input.value = ''

  const validTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp']
  if (!validTypes.includes(file.type)) {
    profileErrMsg.value = '仅支持 jpg/png/gif/webp 格式'
    return
  }
  if (file.size > 2 * 1024 * 1024) {
    profileErrMsg.value = '头像大小不能超过 2MB'
    return
  }

  if (profileAvatarPreview.value?.startsWith('blob:')) {
    URL.revokeObjectURL(profileAvatarPreview.value)
  }
  profileAvatarFile.value = file
  profileAvatarPreview.value = URL.createObjectURL(file)
  profileErrMsg.value = ''
}

async function submitProfile() {
  profileErrMsg.value = ''
  profileSuccessMsg.value = ''

  if (profileAcctno.value && (profileAcctno.value.length < 2 || profileAcctno.value.length > 32)) {
    profileErrMsg.value = '账号长度需 2-32 位'
    return
  }

  const emailChanged = emailEditing.value && profileEmail.value.trim() !== (userStore.userInfo?.email || '')
  if (emailChanged) {
    if (!profileEmail.value.includes('@')) { profileErrMsg.value = '请输入正确的邮箱地址'; return }
    if (!profileEmailCode.value.trim()) { profileErrMsg.value = '请输入邮箱验证码'; return }
  }

  profileLoading.value = true
  try {
    let newAvatarUrl = ''
    let changed = false

    if (profileAvatarFile.value) {
      const uploadRes = await uploadImage(userStore.token, profileAvatarFile.value, 'avatar', {
        brandId: getBrand().id,
        phone: userStore.userInfo?.phone || '',
      })
      newAvatarUrl = uploadRes.url || ''
      profileAvatarFile.value = null
      changed = true
    }

    if (emailChanged) {
      const emailRes = await bindEmailApi({
        token: userStore.token,
        email: profileEmail.value.trim(),
        code: profileEmailCode.value.trim(),
      })
      userStore.updateUserInfo({ email: (emailRes as any).email || profileEmail.value.trim() })
      emailEditing.value = false
      profileEmailCode.value = ''
      changed = true
    }

    const changedFields: Record<string, string> = {}
    if (profileNickname.value !== (userStore.userInfo?.username || '')) changedFields.nickname = profileNickname.value
    if (profileAcctno.value !== (userStore.userInfo?.acctno || '')) changedFields.acctno = profileAcctno.value
    if (newAvatarUrl) changedFields.avatars = newAvatarUrl

    if (Object.keys(changedFields).length > 0) {
      const res = await updateProfileApi({ token: userStore.token, ...changedFields })
      userStore.updateUserInfo({
        username: (res as any).nickname || userStore.userInfo?.username,
        acctno: (res as any).acctno || userStore.userInfo?.acctno,
        avatars: (res as any).avatars || newAvatarUrl || userStore.userInfo?.avatars,
      })
      changed = true
    }

    profileSuccessMsg.value = changed ? '保存成功' : '没有需要修改的内容'
    if (changed) setTimeout(() => closeProfileModal(), 800)
  } catch (err: unknown) {
    profileErrMsg.value = err instanceof Error ? err.message : '保存失败'
  } finally {
    profileLoading.value = false
  }
}

const showRechargeModal = ref(false)
const rechargeTab = ref<'online' | 'card'>('card')
const rechargeCardKey = ref('')
const rechargeLoading = ref(false)
const rechargeErrMsg = ref('')
const rechargeSuccessMsg = ref('')

const hasOnlinePay = computed(() => brand.pay_channel !== 'none' && (brand.pay_methods || []).length > 0)
const hasWechat = computed(() => (brand.pay_methods || []).includes('wechat'))
const hasAlipay = computed(() => (brand.pay_methods || []).includes('alipay'))

const packages = ref<PayPackage[]>([])
const packagesLoading = ref(false)
const selectedPkg = ref<PayPackage | null>(null)
const selectedPayMethod = ref<'wechat' | 'alipay'>('wechat')
const payQrUrl = ref('')
const payOrderNo = ref('')
const showQrModal = ref(false)
const payStatus = ref<'pending' | 'paid' | 'failed'>('pending')
let pollTimer: ReturnType<typeof setInterval> | null = null

function handleRecharge() {
  showUserMenu.value = false
  rechargeCardKey.value = ''
  rechargeErrMsg.value = ''
  rechargeSuccessMsg.value = ''
  rechargeTab.value = hasOnlinePay.value ? 'online' : 'card'
  showRechargeModal.value = true
  if (hasOnlinePay.value && packages.value.length === 0) loadPackages()
}

function closeRechargeModal() {
  showRechargeModal.value = false
  if (forceExpired.value) {
    showExpired.value = true
  }
}

async function submitRecharge() {
  rechargeErrMsg.value = ''
  rechargeSuccessMsg.value = ''
  if (!rechargeCardKey.value.trim()) { rechargeErrMsg.value = '请输入卡密'; return }

  rechargeLoading.value = true
  try {
    const res = await redeemCardInnerApi({
      token: userStore.token,
      card_key: rechargeCardKey.value.trim(),
    })
    const cardType = (res as any).card_type || ''
    const expireAt = (res as any).vip_expire_at || ''
    let msg = '充值成功！'
    if (cardType) msg += ` (${cardType})`
    if (expireAt) {
      const d = new Date(expireAt)
      msg += `\n到期时间：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    }
    rechargeSuccessMsg.value = msg
    rechargeCardKey.value = ''
    if (forceExpired.value) {
      forceExpired.value = false
      showExpired.value = false
      if (userStore.token) startHeartbeat(userStore.token)
    }
    setTimeout(() => closeRechargeModal(), 3000)
  } catch (err: unknown) {
    rechargeErrMsg.value = err instanceof Error ? err.message : '充值失败'
  } finally {
    rechargeLoading.value = false
  }
}

async function loadPackages() {
  packagesLoading.value = true
  try {
    const { appId } = await getAppCredentials()
    logger.log('pay', '请求套餐列表', { appId })
    const res = await fetchPackages(userStore.token, appId)
    logger.log('pay', '套餐列表响应', res)
    packages.value = res.items || []
    logger.log('pay', '解析后套餐数量', packages.value.length)
  } catch (err) {
    logger.error('pay', '套餐列表请求失败', err)
    packages.value = []
  } finally { packagesLoading.value = false }
}

function formatPrice(price: number) {
  return (price / 100).toFixed(2)
}

function formatDuration(seconds?: number) {
  if (!seconds) return ''
  const days = Math.floor(seconds / 86400)
  if (days >= 365 && days % 365 === 0) return `${days / 365}年`
  if (days >= 30 && days % 30 === 0) return `${days / 30}个月`
  return `${days}天`
}

function selectPackage(pkg: PayPackage) {
  selectedPkg.value = pkg
  if (hasWechat.value) selectedPayMethod.value = 'wechat'
  else if (hasAlipay.value) selectedPayMethod.value = 'alipay'
}

async function startPay() {
  if (!selectedPkg.value) return
  rechargeErrMsg.value = ''
  rechargeLoading.value = true
  try {
    const { appId } = await getAppCredentials()
    const payMethod = brand.pay_channel === 'hupijiao'
      ? `hupijiao_${selectedPayMethod.value}`
      : selectedPayMethod.value
    const res = await createOrder(userStore.token, appId, {
      card_group_id: selectedPkg.value.id,
      payment_method: payMethod,
    })
    logger.log('pay', '创建订单响应', res)
    payOrderNo.value = res.order_no

    const rawUrl = res.pay_url || res.code_url || res.qr_code || ''
    if (!rawUrl) { rechargeErrMsg.value = '服务端未返回支付链接'; return }
    payQrUrl.value = await QRCode.toDataURL(rawUrl, { width: 280, margin: 2 })
    payStatus.value = 'pending'
    showQrModal.value = true
    startPayPolling()
  } catch (err: unknown) {
    rechargeErrMsg.value = err instanceof Error ? err.message : '创建订单失败'
  } finally {
    rechargeLoading.value = false
  }
}

function startPayPolling() {
  stopPayPolling()
  pollTimer = setInterval(async () => {
    try {
      const res = await queryOrder(userStore.token, payOrderNo.value)
      logger.log('pay', '订单状态', { status: res.status, order_no: res.order_no, raw: res })
      if (res.status === 'paid') {
        payStatus.value = 'paid'
        stopPayPolling()
        const update: Record<string, unknown> = {}
        if ((res as any).vip_expire_at) update.vip_expire_at = (res as any).vip_expire_at
        if ((res as any).fen != null) update.fen = (res as any).fen
        if (Object.keys(update).length) userStore.updateUserInfo(update as any)
        if (forceExpired.value) {
          forceExpired.value = false
          showExpired.value = false
        }
        if (userStore.token) startHeartbeat(userStore.token)
        setTimeout(() => { closeQrModal(); closeRechargeModal() }, 3000)
      } else if (res.status === 'failed') {
        payStatus.value = 'failed'
        stopPayPolling()
      }
    } catch { /* polling error, continue */ }
  }, 3000)
}

function stopPayPolling() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
}

function closeQrModal() {
  showQrModal.value = false
  stopPayPolling()
}

const showChangePwdModal = ref(false)
const cpOldPwd = ref('')
const cpNewPwd = ref('')
const cpConfirmPwd = ref('')
const cpLoading = ref(false)
const cpErrMsg = ref('')
const cpSuccessMsg = ref('')

function handleChangePassword() {
  showUserMenu.value = false
  cpOldPwd.value = ''
  cpNewPwd.value = ''
  cpConfirmPwd.value = ''
  cpErrMsg.value = ''
  cpSuccessMsg.value = ''
  showChangePwdModal.value = true
}

async function submitChangePassword() {
  cpErrMsg.value = ''
  cpSuccessMsg.value = ''
  if (!cpOldPwd.value) { cpErrMsg.value = '请输入原密码'; return }
  if (!cpNewPwd.value || cpNewPwd.value.length < 6) { cpErrMsg.value = '新密码至少6位'; return }
  if (cpNewPwd.value !== cpConfirmPwd.value) { cpErrMsg.value = '两次密码不一致'; return }

  cpLoading.value = true
  try {
    await changePasswordApi({
      token: userStore.token,
      old_password: cpOldPwd.value,
      new_password: cpNewPwd.value,
    })
    cpSuccessMsg.value = '密码修改成功'
    setTimeout(() => { showChangePwdModal.value = false }, 800)
  } catch (err: unknown) {
    cpErrMsg.value = err instanceof Error ? err.message : '修改失败'
  } finally {
    cpLoading.value = false
  }
}

const showUnbindModal = ref(false)
const unbindLoading = ref(false)
const unbindErrMsg = ref('')
const unbindSuccessMsg = ref('')

function handleUnbindDevice() {
  showUserMenu.value = false
  unbindErrMsg.value = ''
  unbindSuccessMsg.value = ''
  showUnbindModal.value = true
}

async function submitUnbind() {
  unbindErrMsg.value = ''
  unbindSuccessMsg.value = ''
  unbindLoading.value = true
  try {
    const deviceId = await import('../../utils/device').then(m => m.getDeviceId())
    await unbindDeviceInnerApi({ token: userStore.token, device_id: deviceId })
    unbindSuccessMsg.value = '解绑成功，即将退出登录...'
    setTimeout(async () => {
      showUnbindModal.value = false
      stopHeartbeat()
      userStore.logout()
      await switchToLoginLayout(router)
    }, 1500)
  } catch (err: unknown) {
    unbindErrMsg.value = err instanceof Error ? err.message : '解绑失败'
  } finally {
    unbindLoading.value = false
  }
}
</script>

<template>
  <div class="layout-root">
    <TitleBar variant="full" :title="pageTitle" :contact-float-visible="showContactFloat" @restore-contact="restoreContactFloat" @open-disclaimer="openDisclaimer" />

    <div class="layout-body">
      <aside class="sidebar" :class="{ 'sidebar--collapsed': collapsed }">
        <div class="sidebar-user">
          <div class="sidebar-avatar">
            <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="avatar-img" alt="" />
            <span v-else>{{ displayName.charAt(0) }}</span>
          </div>
          <div class="sidebar-user-meta">
            <strong>{{ displayName }}</strong>
            <span>{{ userStore.isLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
        </div>

        <nav class="sidebar-nav">
          <button
            v-for="item in menuItems"
            :key="item.path"
            type="button"
            class="nav-item"
            :class="{ active: route.path === item.path }"
            :title="collapsed ? item.label : undefined"
            @click="handleNavigate(item.path)"
          >
            <i :class="item.icon"></i>
            <span class="nav-label">{{ item.label }}</span>
          </button>
        </nav>

        <button type="button" class="collapse-btn" :title="collapsed ? '展开菜单' : '折叠菜单'" @click="toggleSidebar">
          <i class="pi" :class="collapsed ? 'pi-angle-double-right' : 'pi-angle-double-left'"></i>
          <span class="nav-label">折叠菜单</span>
        </button>
      </aside>

      <section class="main-area">
        <div class="content-header">
          <div class="header-left">
            <div class="page-title">{{ pageTitle }}</div>
            <div class="page-subtitle">欢迎回来，开始今天的任务吧</div>
          </div>
          <div class="header-right">
            <button v-if="brand.tutorial_url" class="ch-tutorial-btn" @click="openUrl(brand.tutorial_url)">
              <i class="pi pi-book"></i> 使用教程
            </button>
            <div v-if="brand.checkin_reward_value > 0" class="ch-checkin-wrap">
              <button class="ch-checkin-btn" :class="{ 'ch-checkin-done': !canCheckin }" :disabled="checkinLoading || !canCheckin" @click="handleCheckin">
                <i class="pi" :class="canCheckin ? 'pi-sun' : 'pi-check-circle'"></i>
                {{ checkinLoading ? '签到中…' : canCheckin ? '签到' : '已签到' }}
              </button>
              <Transition name="fade">
                <span v-if="showCheckinHint" class="ch-checkin-hint">签到可获得 {{ rewardSummary }}</span>
              </Transition>
              <Transition name="fade">
                <span v-if="checkinMsg" class="ch-checkin-result">{{ checkinMsg }}</span>
              </Transition>
            </div>
            <button class="ch-recharge-btn" @click="handleRecharge">
              <i class="pi pi-bolt"></i>
              <span>充值中心</span>
            </button>
            <div class="user-card-wrap">
              <button type="button" class="user-card" @click="toggleUserMenu($event)">
                <div class="user-avatar">
                  <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="avatar-img" alt="" />
                  <span v-else>{{ displayName.charAt(0) }}</span>
                </div>
                <div class="user-meta">
                  <strong>{{ displayName }}</strong>
                  <span>当前账号</span>
                </div>
                <i class="pi pi-chevron-down user-card-arrow" :class="{ 'arrow--open': showUserMenu }"></i>
              </button>

              <Transition name="dropdown">
                <div v-if="showUserMenu" class="user-dropdown" @click.stop>
                  <div class="ud-header">
                    <div class="ud-avatar">
                      <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="avatar-img" alt="" />
                      <span v-else>{{ displayName.charAt(0) }}</span>
                    </div>
                    <div class="ud-info">
                      <strong>{{ displayName }}</strong>
                      <span>{{ userStore.userInfo?.phone || userStore.userInfo?.email || '未绑定手机/邮箱' }}</span>
                    </div>
                  </div>
                  <div class="ud-divider"></div>
                  <button class="ud-item" @click="handleEditProfile">
                    <i class="pi pi-user-edit"></i>
                    编辑资料
                  </button>
                  <button class="ud-item" @click="handleRecharge">
                    <i class="pi pi-credit-card"></i>
                    卡密充值
                  </button>
                  <button class="ud-item" @click="handleChangePassword">
                    <i class="pi pi-lock"></i>
                    修改密码
                  </button>
                  <button class="ud-item" @click="handleUnbindDevice">
                    <i class="pi pi-link"></i>
                    解绑设备
                  </button>
                  <div class="ud-divider"></div>
                  <button class="ud-item ud-item--danger" @click="handleLogout">
                    <i class="pi pi-sign-out"></i>
                    退出登录
                  </button>
                </div>
              </Transition>
            </div>
          </div>
        </div>

        <main class="content-shell">
          <RouterView v-slot="{ Component }">
            <KeepAlive>
              <component :is="Component" />
            </KeepAlive>
          </RouterView>
        </main>
      </section>
    </div>

    <!-- 充值中心弹窗 -->
    <Transition name="modal">
      <div v-if="showRechargeModal" class="modal-overlay" @click.self="closeRechargeModal">
        <div class="modal-box modal-box--recharge">
          <div class="modal-header">
            <h3><i class="pi pi-bolt" style="color:var(--app-primary);margin-right:6px"></i>充值中心</h3>
            <button type="button" class="modal-close" @click="closeRechargeModal">
              <i class="pi pi-times"></i>
            </button>
          </div>

          <div v-if="hasOnlinePay" class="rc-tabs">
            <button class="rc-tab" :class="{ active: rechargeTab === 'online' }" @click="rechargeTab = 'online'">
              <i class="pi pi-shopping-cart"></i> 在线支付
            </button>
            <button class="rc-tab" :class="{ active: rechargeTab === 'card' }" @click="rechargeTab = 'card'">
              <i class="pi pi-credit-card"></i> 卡密兑换
            </button>
          </div>

          <!-- 在线支付 -->
          <div v-if="rechargeTab === 'online' && hasOnlinePay" class="rc-body">
            <div v-if="packagesLoading" class="rc-loading">
              <i class="pi pi-spin pi-spinner"></i> 加载套餐中…
            </div>
            <div v-else-if="packages.length === 0" class="rc-empty">暂无可购买的套餐</div>
            <template v-else>
              <div class="rc-packages">
                <div
                  v-for="pkg in packages" :key="pkg.id"
                  class="rc-pkg-card"
                  :class="{ selected: selectedPkg?.id === pkg.id }"
                  @click="selectPackage(pkg)"
                >
                  <div class="rc-pkg-name">{{ pkg.name }}</div>
                  <div class="rc-pkg-desc">{{ pkg.description || formatDuration(pkg.duration_seconds) }}</div>
                  <div class="rc-pkg-price">
                    <span class="rc-price-symbol">¥</span>
                    <span class="rc-price-value">{{ formatPrice(pkg.price) }}</span>
                  </div>
                </div>
              </div>

              <div v-if="selectedPkg" class="rc-pay-methods">
                <span class="rc-pay-label">支付方式</span>
                <div class="rc-pay-options">
                  <label v-if="hasWechat" class="rc-pay-opt" :class="{ active: selectedPayMethod === 'wechat' }">
                    <input type="radio" v-model="selectedPayMethod" value="wechat" />
                    <i class="pi pi-microsoft" style="color:#07c160"></i>
                    <span>微信支付</span>
                  </label>
                  <label v-if="hasAlipay" class="rc-pay-opt" :class="{ active: selectedPayMethod === 'alipay' }">
                    <input type="radio" v-model="selectedPayMethod" value="alipay" />
                    <i class="pi pi-wallet" style="color:#1677ff"></i>
                    <span>支付宝</span>
                  </label>
                </div>
              </div>

              <Transition name="fade">
                <div v-if="rechargeErrMsg" class="modal-msg modal-msg--err">
                  <i class="pi pi-exclamation-circle"></i>{{ rechargeErrMsg }}
                </div>
              </Transition>

              <button class="rc-pay-btn" :disabled="!selectedPkg || rechargeLoading" @click="startPay">
                <i v-if="rechargeLoading" class="pi pi-spin pi-spinner"></i>
                <template v-else>
                  <i class="pi pi-bolt"></i>
                  立即支付 {{ selectedPkg ? '¥' + formatPrice(selectedPkg.price) : '' }}
                </template>
              </button>
            </template>
          </div>

          <!-- 卡密兑换 -->
          <form v-if="rechargeTab === 'card'" class="modal-body" @submit.prevent="submitRecharge">
            <div class="modal-field">
              <label>当前账号</label>
              <div class="modal-acctno">{{ userStore.userInfo?.phone || userStore.userInfo?.username || '-' }}</div>
            </div>
            <div class="modal-field">
              <label>卡密</label>
              <InputText v-model="rechargeCardKey" placeholder="请输入卡密" class="modal-input" />
            </div>
            <Transition name="fade">
              <div v-if="rechargeErrMsg" class="modal-msg modal-msg--err">
                <i class="pi pi-exclamation-circle"></i>{{ rechargeErrMsg }}
              </div>
              <div v-else-if="rechargeSuccessMsg" class="modal-msg modal-msg--ok">
                <i class="pi pi-check-circle"></i>{{ rechargeSuccessMsg }}
              </div>
            </Transition>
            <Button type="submit" label="充 值" :loading="rechargeLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 支付等待弹窗 -->
    <Transition name="modal">
      <div v-if="showQrModal" class="modal-overlay" @click.self="closeQrModal">
        <div class="modal-box qr-modal">
          <div class="modal-header">
            <h3>{{ selectedPayMethod === 'wechat' ? '微信支付' : '支付宝支付' }}</h3>
            <button type="button" class="modal-close" @click="closeQrModal"><i class="pi pi-times"></i></button>
          </div>
          <div class="qr-body">
            <div v-if="payStatus === 'pending'" class="qr-wrap">
              <img :src="payQrUrl" alt="支付二维码" class="qr-img" />
              <p class="qr-hint">请使用{{ selectedPayMethod === 'wechat' ? '微信' : '支付宝' }}扫码支付</p>
              <p class="qr-amount">¥{{ selectedPkg ? formatPrice(selectedPkg.price) : '' }}</p>
              <div class="qr-polling"><i class="pi pi-spin pi-spinner"></i> 等待支付结果…</div>
            </div>
            <div v-else-if="payStatus === 'paid'" class="qr-result qr-result--ok">
              <i class="pi pi-check-circle"></i>
              <p>支付成功！</p>
            </div>
            <div v-else class="qr-result qr-result--fail">
              <i class="pi pi-times-circle"></i>
              <p>支付失败，请重试</p>
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 编辑资料弹窗 -->
    <Transition name="modal">
      <div v-if="showProfileModal" class="modal-overlay" @click.self="closeProfileModal">
        <div class="modal-box modal-box--profile">
          <div class="modal-header">
            <h3>编辑资料</h3>
            <button type="button" class="modal-close" @click="closeProfileModal">
              <i class="pi pi-times"></i>
            </button>
          </div>
          <form class="modal-body" @submit.prevent="submitProfile">
            <div class="profile-avatar-area">
              <div class="profile-avatar-wrap" @click="triggerAvatarSelect">
                <img v-if="profileAvatarPreview" :src="profileAvatarPreview" class="profile-avatar-img" alt="头像" />
                <div v-else class="profile-avatar-placeholder">{{ displayName.charAt(0) }}</div>
                <div class="profile-avatar-overlay">
                  <i class="pi pi-camera"></i>
                </div>
              </div>
              <input ref="avatarInputRef" type="file" accept="image/jpeg,image/png,image/gif,image/webp" style="display:none" @change="onAvatarSelected" />
              <span class="profile-avatar-hint">点击更换头像</span>
            </div>

            <div class="modal-field">
              <label>昵称</label>
              <InputText v-model="profileNickname" placeholder="请输入昵称" class="modal-input" maxlength="128" />
            </div>
            <div class="modal-field">
              <label>账号</label>
              <InputText v-model="profileAcctno" placeholder="可用来登录的账号（2-32位）" class="modal-input" maxlength="32" />
            </div>
            <div class="modal-field">
              <label>邮箱 <span class="field-hint">可用于找回密码</span></label>
              <template v-if="!emailEditing && userStore.userInfo?.email">
                <div class="email-bound">
                  <span>{{ userStore.userInfo.email }}</span>
                  <button type="button" class="email-change-btn" @click="emailEditing = true; profileEmail = ''">更换</button>
                </div>
              </template>
              <template v-else>
                <InputText v-model="profileEmail" placeholder="请输入邮箱地址" class="modal-input" type="email" />
                <div class="email-code-row">
                  <InputText v-model="profileEmailCode" placeholder="验证码" class="modal-input email-code-input" maxlength="6" />
                  <Button
                    type="button"
                    :label="emailBtnText"
                    :disabled="emailCooldown > 0"
                    :loading="emailSending"
                    class="email-code-btn"
                    @click="handleSendEmailCode"
                  />
                </div>
              </template>
            </div>

            <Transition name="fade">
              <div v-if="profileErrMsg" class="modal-msg modal-msg--err">
                <i class="pi pi-exclamation-circle"></i>{{ profileErrMsg }}
              </div>
              <div v-else-if="profileSuccessMsg" class="modal-msg modal-msg--ok">
                <i class="pi pi-check-circle"></i>{{ profileSuccessMsg }}
              </div>
            </Transition>

            <Button type="submit" label="保存" :loading="profileLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 修改密码弹窗 -->
    <Transition name="modal">
      <div v-if="showChangePwdModal" class="modal-overlay" @click.self="showChangePwdModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>修改密码</h3>
            <button type="button" class="modal-close" @click="showChangePwdModal = false">
              <i class="pi pi-times"></i>
            </button>
          </div>
          <form class="modal-body" @submit.prevent="submitChangePassword">
            <div class="modal-field">
              <label>原密码</label>
              <InputText v-model="cpOldPwd" placeholder="请输入原密码" class="modal-input" type="password" />
            </div>
            <div class="modal-field">
              <label>新密码</label>
              <InputText v-model="cpNewPwd" placeholder="请输入新密码（至少6位）" class="modal-input" type="password" />
            </div>
            <div class="modal-field">
              <label>确认新密码</label>
              <InputText v-model="cpConfirmPwd" placeholder="请再次输入新密码" class="modal-input" type="password" />
            </div>

            <Transition name="fade">
              <div v-if="cpErrMsg" class="modal-msg modal-msg--err">
                <i class="pi pi-exclamation-circle"></i>{{ cpErrMsg }}
              </div>
              <div v-else-if="cpSuccessMsg" class="modal-msg modal-msg--ok">
                <i class="pi pi-check-circle"></i>{{ cpSuccessMsg }}
              </div>
            </Transition>

            <Button type="submit" label="确认修改" :loading="cpLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 解绑设备弹窗 -->
    <Transition name="modal">
      <div v-if="showUnbindModal" class="modal-overlay" @click.self="showUnbindModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>解绑当前设备</h3>
            <button type="button" class="modal-close" @click="showUnbindModal = false">
              <i class="pi pi-times"></i>
            </button>
          </div>
          <div class="modal-body" style="text-align:center">
            <p style="font-size:0.9rem;color:#475569;margin:0 0 20px;line-height:1.6">{{ getUnbindTip() }}</p>

            <Transition name="fade">
              <div v-if="unbindErrMsg" class="modal-msg modal-msg--err">
                <i class="pi pi-exclamation-circle"></i>{{ unbindErrMsg }}
              </div>
              <div v-else-if="unbindSuccessMsg" class="modal-msg modal-msg--ok">
                <i class="pi pi-check-circle"></i>{{ unbindSuccessMsg }}
              </div>
            </Transition>

            <Button label="确认解绑" :loading="unbindLoading" class="modal-submit" style="margin-top:8px" @click="submitUnbind" />
          </div>
        </div>
      </div>
    </Transition>

    <!-- 到期续费弹窗 -->
    <Transition name="modal">
      <div v-if="showExpired" class="modal-overlay">
        <div class="modal-box">
          <div class="modal-header">
            <h3>使用到期</h3>
          </div>
          <div class="modal-body" style="text-align:center">
            <div class="expired-icon">
              <i class="pi pi-clock"></i>
            </div>
            <p class="expired-msg">{{ expiredMsg }}</p>
            <div style="display:flex;gap:10px">
              <Button label="卡密充值" class="modal-submit" style="flex:1" @click="showExpired = false; handleRecharge()" />
              <Button label="退出登录" class="modal-submit" severity="secondary" style="flex:1;background:#64748b !important" @click="forceExpired = false; showExpired = false; handleLogout()" />
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 账号禁用遮罩 -->
    <div v-if="showBanned" class="banned-overlay">
      <div class="banned-box">
        <div class="banned-icon">
          <i class="pi pi-ban"></i>
        </div>
        <h2 class="banned-title">{{ bannedTitle }}</h2>
        <p class="banned-msg">{{ bannedMsg }}</p>
        <p class="banned-countdown">{{ bannedCountdown }} 秒后自动退出登录</p>
      </div>
    </div>

    <!-- Floating Contact Button (right edge, vertical) -->
    <Transition name="contact-float">
      <div v-if="showContactFloat && hasContactImages" class="contact-float" :class="{ 'contact-highlight': contactHighlight }">
        <button class="contact-float-main" @click="showContactModal = true">
          <i class="pi pi-comments"></i>
          <span>联</span><span>系</span><span>我</span><span>们</span>
        </button>
        <button class="contact-float-x" @click="closeContactFloat"><i class="pi pi-times"></i></button>
      </div>
    </Transition>

    <!-- Contact Images Modal -->
    <Transition name="modal">
      <div v-if="showContactModal" class="contact-modal-overlay" @click.self="showContactModal = false">
        <div class="contact-modal-box">
          <div class="contact-modal-header">
            <h3>联系我们</h3>
            <button type="button" class="contact-modal-close" @click="showContactModal = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="contact-modal-body">
            <img v-for="(img, idx) in cachedImages" :key="idx" :src="img" class="contact-img" alt="联系方式" />
          </div>
        </div>
      </div>
    </Transition>

    <!-- 版本更新弹窗 -->
    <Transition name="modal">
      <div v-if="showUpdateDialog && updateInfo?.has_update" class="modal-overlay update-overlay">
        <div class="update-dialog">
          <div class="update-header">
            <h3><i class="pi pi-arrow-circle-up" style="color:var(--app-primary);margin-right:6px"></i>发现新版本 V{{ updateInfo.latest_version }}</h3>
            <button v-if="!updateInfo.force_update" type="button" class="modal-close" @click="dismissUpdate"><i class="pi pi-times"></i></button>
          </div>
          <div class="update-body">
            <div class="update-left">
              <div
                v-for="(u, idx) in updateInfo.updates" :key="u.version"
                class="update-ver-item" :class="{ active: selectedUpdateIdx === idx }"
                @click="selectedUpdateIdx = idx"
              >
                <div class="update-ver-name">版本 {{ u.version }}</div>
                <div v-if="u.force_update" class="update-ver-force">强制更新</div>
                <div class="update-ver-date">{{ u.created_at }}</div>
              </div>
            </div>
            <div class="update-right">
              <template v-if="updateInfo.updates[selectedUpdateIdx]">
                <div class="update-detail-title">
                  版本 {{ updateInfo.updates[selectedUpdateIdx].version }} 更新内容如下
                </div>
                <div class="update-detail-content" v-html="updateInfo.updates[selectedUpdateIdx].description || '暂无更新说明'"></div>
              </template>
            </div>
          </div>
          <div class="update-footer">
            <div v-if="downloadStatus === 'downloading'" class="update-progress">
              <div class="update-progress-bar"><div class="update-progress-fill" :style="{ width: downloadProgress + '%' }"></div></div>
              <span>下载中 {{ downloadProgress }}%</span>
            </div>
            <div v-else-if="downloadStatus === 'verifying'" class="update-progress">
              <i class="pi pi-spin pi-spinner"></i> <span>校验文件完整性…</span>
            </div>
            <div v-else-if="downloadStatus === 'installing'" class="update-progress">
              <i class="pi pi-spin pi-spinner"></i> <span>正在安装…</span>
            </div>
            <div v-else-if="downloadStatus === 'error'" class="update-err">{{ downloadError }}</div>
            <template v-else>
              <button v-if="!updateInfo.force_update" class="vm-btn vm-btn--outline" @click="dismissUpdate">稍后提醒</button>
              <button class="vm-btn vm-btn--primary" @click="applyUpdate"><i class="pi pi-download"></i> 立即更新</button>
            </template>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 免责声明弹窗 -->
    <DisclaimerDialog
      :visible="showDisclaimerModal"
      :show-accept-button="true"
      @close="closeDisclaimer"
      @accept="acceptDisclaimer"
    />
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.layout-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #eef2f7;
  animation: page-fade-in 0.35s ease-out;
}

.layout-body {
  display: flex;
  flex: 1;
  min-height: 0;
}

/* ── Sidebar ── */
.sidebar {
  width: 240px;
  flex-shrink: 0;
  background: #102136;
  color: #e2e8f0;
  display: flex;
  flex-direction: column;
  padding: 0 12px 12px;
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.sidebar--collapsed {
  width: 64px;
}

.sidebar--collapsed .sidebar-user-meta,
.sidebar--collapsed .nav-label {
  opacity: 0;
  width: 0;
  overflow: hidden;
  white-space: nowrap;
  pointer-events: none;
}

.sidebar--collapsed .sidebar-user {
  justify-content: center;
  padding: 18px 0;
  gap: 0;
}

.sidebar--collapsed .nav-item,
.sidebar--collapsed .collapse-btn {
  justify-content: center;
  padding: 0;
}

.sidebar-user {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  margin-bottom: 12px;
  transition: all 0.25s;
}

.sidebar-avatar {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.95rem;
  flex-shrink: 0;
  overflow: hidden;
}

.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.sidebar-user-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  transition: opacity 0.2s;
}

.sidebar-user-meta strong {
  color: #fff;
  font-size: 0.92rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-user-meta span {
  color: #94a3b8;
  font-size: 0.75rem;
}

.sidebar-nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  height: 42px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: #cbd5e1;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 14px;
  cursor: pointer;
  font-size: 0.88rem;
  transition: all 0.15s ease;
  width: 100%;
  text-align: left;
}

.nav-item i {
  font-size: 1rem;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.nav-label {
  transition: opacity 0.2s;
  white-space: nowrap;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(45, 212, 191, 0.22), rgba(13, 148, 136, 0.32));
  color: #fff;
  box-shadow: inset 0 0 0 1px rgba(94, 234, 212, 0.12);
}

/* ── Collapse Button ── */
.collapse-btn {
  height: 42px;
  border: none;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px;
  cursor: pointer;
  font-size: 0.85rem;
  transition: all 0.15s;
  width: 100%;
  margin-top: 8px;
}

.collapse-btn i {
  font-size: 1rem;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
  transition: transform 0.25s;
}

.collapse-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #e2e8f0;
}

/* ── Main Area ── */
.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.content-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 64px;
  padding: 0 24px;
  background: #fff;
  border-bottom: 1px solid #e2e8f0;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.page-title {
  font-size: 1rem;
  font-weight: 700;
  color: #0f172a;
}

.page-subtitle {
  font-size: 0.78rem;
  color: #64748b;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.ch-tutorial-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 14px;
  border: 1px solid var(--app-primary-border, #86efac);
  border-radius: 6px;
  background: var(--app-primary-light, #f0fdf4);
  color: var(--app-primary, #22c55e);
  font-size: 0.78rem;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}
.ch-tutorial-btn:hover {
  background: var(--app-primary-light-hover, #dcfce7);
  border-color: var(--app-primary, #22c55e);
}

.ch-checkin-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
}
.ch-checkin-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 14px;
  border: 1px solid var(--app-primary-border, #86efac);
  border-radius: 6px;
  background: var(--app-primary, #22c55e);
  color: #fff;
  font-size: 0.78rem;
  font-family: inherit;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}
.ch-checkin-btn:hover:not(:disabled) {
  background: var(--app-primary-hover, #16a34a);
  border-color: var(--app-primary-hover, #16a34a);
}
.ch-checkin-btn:disabled { opacity: 0.65; cursor: default; }
.ch-checkin-done {
  background: var(--app-primary-light, #f0fdf4) !important;
  color: var(--app-primary, #22c55e) !important;
  border-color: var(--app-primary-border, #86efac) !important;
}
.ch-checkin-hint {
  position: absolute;
  top: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  white-space: nowrap;
  padding: 4px 10px;
  background: var(--app-primary, #22c55e);
  color: #fff;
  font-size: 0.7rem;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(34, 197, 94, 0.3);
  z-index: 10;
  pointer-events: none;
}
.ch-checkin-hint::before {
  content: '';
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 5px solid transparent;
  border-bottom-color: var(--app-primary, #22c55e);
}
.ch-checkin-result {
  position: absolute;
  top: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  white-space: nowrap;
  padding: 4px 10px;
  background: #0f172a;
  color: #fff;
  font-size: 0.7rem;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
  z-index: 10;
  pointer-events: none;
}

/* ── Recharge Center Button ── */
.ch-recharge-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 18px;
  border: none;
  border-radius: 8px;
  background: linear-gradient(135deg, #f59e0b, #f97316);
  color: #fff;
  font-size: 0.8rem;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  box-shadow: 0 2px 10px rgba(245, 158, 11, 0.4);
  transition: all 0.2s;
  letter-spacing: 0.03em;
}
.ch-recharge-btn:hover {
  background: linear-gradient(135deg, #f97316, #ef4444);
  box-shadow: 0 4px 16px rgba(249, 115, 22, 0.5);
  transform: translateY(-1px);
}
.ch-recharge-btn .pi { font-size: 0.85rem; }

/* ── User Card (clickable) ── */
.user-card-wrap {
  position: relative;
}

.user-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 10px;
  border: none;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s;
}

.user-card:hover {
  background: #f1f5f9;
}

.user-avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.85rem;
  flex-shrink: 0;
  overflow: hidden;
}

.user-meta {
  display: flex;
  flex-direction: column;
  gap: 1px;
  text-align: left;
}

.user-meta strong {
  font-size: 0.88rem;
  color: #0f172a;
}

.user-meta span {
  font-size: 0.72rem;
  color: #64748b;
}

.user-card-arrow {
  font-size: 0.7rem;
  color: #94a3b8;
  margin-left: 2px;
  transition: transform 0.2s;
}

.arrow--open {
  transform: rotate(180deg);
}

/* ── User Dropdown ── */
.user-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  min-width: 220px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 10px 36px rgba(15, 23, 42, 0.14);
  padding: 6px 0;
  z-index: 500;
}

.ud-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px 10px;
}

.ud-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1rem;
  flex-shrink: 0;
  overflow: hidden;
}

.ud-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.ud-info strong {
  font-size: 0.92rem;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ud-info span {
  font-size: 0.75rem;
  color: #94a3b8;
}

.ud-divider {
  height: 1px;
  background: #f1f5f9;
  margin: 4px 0;
}

.ud-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 16px;
  border: none;
  background: none;
  color: #334155;
  font-size: 0.86rem;
  cursor: pointer;
  transition: background 0.12s;
  white-space: nowrap;
}

.ud-item:hover {
  background: #f1f5f9;
}

.ud-item i {
  font-size: 0.92rem;
  width: 18px;
  text-align: center;
  color: #64748b;
}

.ud-item--danger {
  color: #dc2626;
}

.ud-item--danger i {
  color: #dc2626;
}

.ud-item--danger:hover {
  background: #fef2f2;
}

/* ── Dropdown Transition ── */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.content-shell {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 18px;
}

/* ── Recharge Modal ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.45);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-box {
  width: 400px;
  max-width: 90vw;
  background: #fff;
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(15, 23, 42, 0.2);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #f1f5f9;
}

.modal-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: #0f172a;
}

.modal-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: #94a3b8;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.modal-close:hover {
  background: #f1f5f9;
  color: #334155;
}

.modal-body {
  padding: 20px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.modal-field label {
  font-size: 0.82rem;
  font-weight: 600;
  color: #475569;
}

.modal-acctno {
  height: 42px;
  line-height: 42px;
  font-size: 0.9rem;
  color: #0f172a;
  font-weight: 600;
  padding: 0 14px;
  background: #f1f5f9;
  border-radius: 10px;
  border: 1.5px solid #e2e8f0;
}

.modal-field :deep(.modal-input) {
  width: 100%;
  height: 42px;
  font-size: 0.9rem;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  background: #f8fafb;
  padding: 0 14px;
  transition: all 0.2s;
}

.modal-field :deep(.modal-input:focus) {
  border-color: var(--qs-primary-light, #2dd4bf);
  background: #fff;
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.08);
}

.modal-msg {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  padding: 8px 12px;
  border-radius: 8px;
}

.modal-msg--err {
  color: #dc2626;
  background: #fef2f2;
  border: 1px solid #fecaca;
}

.modal-msg--ok {
  color: #0d9488;
  background: #f0fdfa;
  border: 1px solid #99f6e4;
}

.modal-submit {
  width: 100%;
  height: 42px;
  font-size: 0.95rem;
  font-weight: 600;
  border-radius: 21px;
  background: var(--qs-bg-gradient, linear-gradient(135deg, #2dd4bf, #0d9488)) !important;
  border: none !important;
  box-shadow: 0 4px 16px rgba(13, 148, 136, 0.3);
  transition: all 0.2s;
  margin-top: 4px;
}

.modal-submit:hover {
  box-shadow: 0 6px 24px rgba(13, 148, 136, 0.4);
  transform: translateY(-1px);
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .modal-box,
.modal-leave-active .modal-box {
  transition: transform 0.2s ease;
}
.modal-enter-from .modal-box {
  transform: scale(0.95);
}
.modal-leave-to .modal-box {
  transform: scale(0.95);
}

.fade-enter-active,
.fade-leave-active {
  transition: all 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

/* ── Recharge Center Modal ── */
.modal-box--recharge { width: 480px; }

.rc-tabs {
  display: flex;
  border-bottom: 2px solid #f1f5f9;
  padding: 0 24px;
}
.rc-tab {
  flex: 1;
  padding: 12px 0;
  border: none;
  background: none;
  font-size: 0.88rem;
  font-weight: 600;
  font-family: inherit;
  color: #94a3b8;
  cursor: pointer;
  position: relative;
  transition: color 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
.rc-tab.active { color: var(--app-primary, #22c55e); }
.rc-tab.active::after {
  content: '';
  position: absolute;
  bottom: -2px;
  left: 20%;
  right: 20%;
  height: 2px;
  background: var(--app-primary, #22c55e);
  border-radius: 1px;
}

.rc-body { padding: 20px 24px; }
.rc-loading, .rc-empty {
  text-align: center;
  padding: 32px 0;
  color: #94a3b8;
  font-size: 0.85rem;
}

.rc-packages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 10px;
  margin-bottom: 16px;
}
.rc-pkg-card {
  padding: 14px 10px;
  border: 2px solid #e2e8f0;
  border-radius: 12px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: #fff;
}
.rc-pkg-card:hover { border-color: var(--app-primary-border, #86efac); background: #f0fdf4; }
.rc-pkg-card.selected {
  border-color: var(--app-primary, #22c55e);
  background: linear-gradient(135deg, #f0fdf4, #ecfdf5);
  box-shadow: 0 0 0 1px var(--app-primary, #22c55e);
}
.rc-pkg-name {
  font-size: 0.82rem;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 4px;
}
.rc-pkg-desc {
  font-size: 0.7rem;
  color: #64748b;
  margin-bottom: 8px;
}
.rc-pkg-price { color: #ef4444; font-weight: 700; }
.rc-price-symbol { font-size: 0.75rem; }
.rc-price-value { font-size: 1.2rem; }

.rc-pay-methods {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 12px;
  background: #f8fafc;
  border-radius: 10px;
}
.rc-pay-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: #475569;
  white-space: nowrap;
}
.rc-pay-options { display: flex; gap: 10px; }
.rc-pay-opt {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.78rem;
  color: #334155;
  transition: all 0.15s;
  background: #fff;
}
.rc-pay-opt input { display: none; }
.rc-pay-opt.active {
  border-color: var(--app-primary, #22c55e);
  background: #f0fdf4;
  color: var(--app-primary, #22c55e);
  font-weight: 600;
}

.rc-pay-btn {
  width: 100%;
  padding: 12px;
  border: none;
  border-radius: 12px;
  background: linear-gradient(135deg, var(--app-primary, #22c55e), var(--app-primary-hover, #16a34a));
  color: #fff;
  font-size: 0.95rem;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
  box-shadow: 0 4px 14px rgba(34, 197, 94, 0.3);
}
.rc-pay-btn:hover:not(:disabled) {
  box-shadow: 0 6px 20px rgba(34, 197, 94, 0.4);
  transform: translateY(-1px);
}
.rc-pay-btn:disabled { opacity: 0.6; cursor: default; }

/* QR Modal */
.qr-modal { width: 360px; }
.qr-body {
  padding: 24px;
  text-align: center;
}
.qr-wrap { display: flex; flex-direction: column; align-items: center; gap: 12px; }
.qr-img {
  width: 200px;
  height: 200px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  padding: 8px;
}
.qr-hint { font-size: 0.82rem; color: #64748b; margin: 0; }
.qr-browser-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px 0;
  color: var(--app-primary, #22c55e);
}
.qr-browser-hint .pi { font-size: 2.5rem; }
.qr-browser-hint p { margin: 0; font-size: 0.95rem; font-weight: 600; }
.qr-amount { font-size: 1.4rem; font-weight: 800; color: #ef4444; margin: 0; }
.qr-polling {
  font-size: 0.78rem;
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 6px;
}
.qr-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px 0;
}
.qr-result .pi { font-size: 3rem; }
.qr-result p { font-size: 1rem; font-weight: 600; margin: 0; }
.qr-result--ok { color: var(--app-primary, #22c55e); }
.qr-result--fail { color: #ef4444; }

/* ── Profile Modal ── */
.modal-box--profile {
  width: 440px;
}

.profile-avatar-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding-bottom: 8px;
}

.profile-avatar-wrap {
  position: relative;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  cursor: pointer;
  overflow: hidden;
  box-shadow: 0 2px 12px rgba(13, 148, 136, 0.15);
}

.profile-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.profile-avatar-placeholder {
  width: 100%;
  height: 100%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 2rem;
  font-weight: 700;
}

.profile-avatar-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s;
}

.profile-avatar-overlay i {
  color: #fff;
  font-size: 1.3rem;
}

.profile-avatar-wrap:hover .profile-avatar-overlay {
  opacity: 1;
}

.profile-avatar-hint {
  font-size: 0.75rem;
  color: #94a3b8;
}

/* ── Banned Overlay ── */
.banned-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.85);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.banned-box {
  text-align: center;
  padding: 48px 56px;
  background: #fff;
  border-radius: 20px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  max-width: 420px;
}

.banned-icon {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: #fef2f2;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 20px;
}

.banned-icon i {
  font-size: 2rem;
  color: #dc2626;
}

.banned-title {
  margin: 0 0 12px;
  font-size: 1.3rem;
  font-weight: 700;
  color: #dc2626;
}

.banned-msg {
  margin: 0 0 20px;
  font-size: 0.9rem;
  color: #475569;
  line-height: 1.6;
}

.banned-countdown {
  margin: 0;
  font-size: 0.85rem;
  color: #94a3b8;
}

.expired-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: #fef3c7;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.expired-icon i {
  font-size: 1.8rem;
  color: #d97706;
}

.expired-msg {
  font-size: 0.92rem;
  color: #475569;
  line-height: 1.6;
  margin: 0 0 20px;
}

.field-hint {
  font-weight: 400;
  font-size: 0.72rem;
  color: #94a3b8;
  margin-left: 4px;
}

.email-bound {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 42px;
  padding: 0 14px;
  background: #f1f5f9;
  border-radius: 10px;
  border: 1.5px solid #e2e8f0;
  font-size: 0.9rem;
  color: #0f172a;
}

.email-change-btn {
  border: none;
  background: none;
  color: var(--qs-primary, #0d9488);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.15s;
}

.email-change-btn:hover {
  background: rgba(13, 148, 136, 0.08);
}

.email-code-row {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-top: 8px;
}

.email-code-row :deep(.email-code-input) {
  flex: 1;
  min-width: 0;
}

.email-code-btn {
  flex-shrink: 0;
  height: 42px;
  font-size: 0.82rem;
  font-weight: 600;
  border-radius: 10px;
  white-space: nowrap;
  padding: 0 14px;
  background: var(--qs-bg-gradient, linear-gradient(135deg, #2dd4bf, #0d9488)) !important;
  border: none !important;
  color: #fff !important;
  transition: all 0.2s;
}

.email-code-btn:disabled {
  opacity: 0.6;
}

/* ─── Floating Contact Button (right edge vertical) ─── */
.contact-float {
  position: fixed;
  right: 0;
  top: 80%;
  transform: translateY(-50%);
  z-index: 900;
  display: flex;
  flex-direction: column;
  align-items: center;
  border-radius: 8px 0 0 8px;
  background: var(--app-primary, #22c55e);
  box-shadow: -2px 2px 12px rgba(0,0,0,0.12);
  overflow: hidden;
  transition: box-shadow 0.2s;
}
.contact-float:hover {
  box-shadow: -2px 2px 16px rgba(34,197,94,0.3);
}
.contact-float-main {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 10px 7px;
  border: none;
  background: transparent;
  color: #fff;
  font-size: 0.78rem;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  line-height: 1.3;
  letter-spacing: 0.05em;
}
.contact-float-main .pi { font-size: 1rem; margin-bottom: 4px; }
.contact-float-x {
  width: 100%;
  padding: 4px 0;
  border: none;
  border-top: 1px solid rgba(255,255,255,0.25);
  background: transparent;
  color: rgba(255,255,255,0.7);
  font-size: 0.6rem;
  cursor: pointer;
  transition: color 0.15s;
}
.contact-float-x:hover { color: #fff; }
.contact-highlight { animation: contact-pulse 0.6s ease-in-out 3; }
@keyframes contact-pulse {
  0%, 100% { box-shadow: -2px 2px 12px rgba(0,0,0,0.12); }
  50% { box-shadow: -2px 0 0 6px rgba(34,197,94,0.3), -2px 2px 16px rgba(34,197,94,0.2); }
}
.contact-float-enter-active, .contact-float-leave-active { transition: all 0.3s ease; }
.contact-float-enter-from, .contact-float-leave-to { opacity: 0; transform: translateY(-50%) translateX(40px); }

.contact-modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 1100; }
.contact-modal-box { background: #fff; border-radius: 12px; width: 520px; max-height: 80vh; display: flex; flex-direction: column; box-shadow: 0 20px 60px rgba(0,0,0,0.15); overflow: hidden; }
.contact-modal-header { display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; border-bottom: 1px solid #e2e8f0; }
.contact-modal-header h3 { margin: 0; font-size: 1rem; }
.contact-modal-close { width: 28px; height: 28px; border: none; background: #f1f5f9; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center; color: #64748b; }
.contact-modal-close:hover { background: #e2e8f0; color: #0f172a; }
.contact-modal-body { padding: 20px; overflow-y: auto; display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; }
.contact-img { max-width: 200px; max-height: 200px; border-radius: 8px; border: 1px solid #e2e8f0; object-fit: contain; }

/* ── Update Dialog ── */
.update-overlay { z-index: 2000; }
.update-dialog {
  width: 640px; max-width: 90vw; background: #fff; border-radius: 16px;
  box-shadow: 0 20px 60px rgba(15,23,42,0.25); overflow: hidden; display: flex; flex-direction: column;
}
.update-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 20px 24px 16px; border-bottom: 1px solid #f1f5f9;
}
.update-header h3 { margin: 0; font-size: 1.1rem; font-weight: 700; }
.update-body { display: flex; min-height: 260px; max-height: 400px; }
.update-left {
  width: 200px; border-right: 1px solid #f1f5f9; overflow-y: auto; flex-shrink: 0;
  padding: 8px;
}
.update-ver-item {
  padding: 10px 12px; border-radius: 8px; cursor: pointer;
  margin-bottom: 4px; transition: background 0.15s;
}
.update-ver-item:hover { background: #f8fafc; }
.update-ver-item.active {
  background: var(--app-primary-light, #f0fdf4);
  border-left: 3px solid var(--app-primary, #22c55e);
}
.update-ver-name { font-size: 0.88rem; font-weight: 600; color: #0f172a; }
.update-ver-force {
  font-size: 0.68rem; color: #dc2626; background: #fef2f2;
  padding: 1px 6px; border-radius: 3px; display: inline-block; margin-top: 2px;
}
.update-ver-date { font-size: 0.72rem; color: #94a3b8; margin-top: 2px; }
.update-right { flex: 1; padding: 16px 20px; overflow-y: auto; }
.update-detail-title { font-size: 0.92rem; font-weight: 700; color: #0f172a; margin-bottom: 12px; }
.update-detail-content { font-size: 0.85rem; color: #475569; line-height: 1.7; }
.update-detail-content :deep(p) { margin: 4px 0; }
.update-detail-content :deep(ul) { margin: 4px 0; padding-left: 20px; }
.update-footer {
  display: flex; align-items: center; justify-content: flex-end; gap: 10px;
  padding: 16px 24px; border-top: 1px solid #f1f5f9;
}
.update-progress { display: flex; align-items: center; gap: 10px; flex: 1; }
.update-progress-bar {
  flex: 1; height: 6px; background: #e2e8f0; border-radius: 3px; overflow: hidden;
}
.update-progress-fill {
  height: 100%; background: var(--app-primary, #22c55e); border-radius: 3px;
  transition: width 0.3s ease;
}
.update-err { color: #dc2626; font-size: 0.82rem; flex: 1; }
.vm-btn {
  display: inline-flex; align-items: center; gap: 6px; padding: 8px 18px;
  border: 1.5px solid #e2e8f0; border-radius: 8px; background: #fff;
  font-size: 0.85rem; font-weight: 600; font-family: inherit; cursor: pointer;
}
.vm-btn--primary { background: var(--app-primary, #22c55e); color: #fff; border-color: var(--app-primary, #22c55e); }
.vm-btn--outline { background: #fff; color: #64748b; }
</style>
