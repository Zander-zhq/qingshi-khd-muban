<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from './TitleBar.vue'
import { useAuth } from '../../composables/useAuth'
import { useProfile } from '../../composables/useProfile'
import { useChangePassword } from '../../composables/useChangePassword'
import { useUnbindDevice } from '../../composables/useUnbindDevice'
import { getUnbindTip, getAppCredentials } from '../../utils/config'
import { fetchPackages, createOrder, queryOrder } from '../../api/pay'
import type { PayPackage } from '../../api/pay'
import QRCode from 'qrcode'
import { redeemCardInnerApi } from '../../api/auth'
import { startHeartbeat } from '../../utils/heartbeat'
import { getBrand, getBrandLogo } from '../../brand'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useContact } from '../../composables/useContact'
import { useVersionUpdate } from '../../composables/useVersionUpdate'
import { useCheckin } from '../../composables/useCheckin'

const brand = getBrand()
const brandLogo = getBrandLogo()
const route = useRoute()

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

const {
  userStore, displayName,
  showBanned, bannedTitle, bannedMsg, bannedCountdown,
  showExpired, expiredMsg, forceExpired,
  initHeartbeat, destroyHeartbeat, handleLogout,
} = useAuth()

const {
  showProfileModal, profileNickname, profileAcctno, profileEmail, profileEmailCode,
  emailEditing, emailSending, emailCooldown, emailBtnText,
  profileAvatarPreview, profileLoading, profileErrMsg, profileSuccessMsg,
  avatarInputRef,
  openProfileModal, closeProfileModal, triggerAvatarSelect, onAvatarSelected,
  handleSendEmailCode, submitProfile,
} = useProfile()

const {
  showChangePwdModal, cpOldPwd, cpNewPwd, cpConfirmPwd, cpLoading, cpErrMsg, cpSuccessMsg,
  openChangePwdModal, submitChangePassword,
} = useChangePassword()

const {
  showUnbindModal, unbindLoading, unbindErrMsg, unbindSuccessMsg,
  openUnbindModal, submitUnbind,
} = useUnbindDevice()

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
    const res = await fetchPackages(userStore.token, appId)
    packages.value = res.items || []
  } catch { packages.value = [] }
  finally { packagesLoading.value = false }
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

interface SubItem { label: string; icon?: string; path: string }
interface MenuItem { label: string; icon: string; path?: string; children?: SubItem[] }

const menuItems: MenuItem[] = [
  { label: '仪表盘', icon: 'pi pi-home', path: '/main/dashboard' },
  ...(import.meta.env.DEV ? [{
    label: '开发工具', icon: 'pi pi-wrench',
    children: [
      { label: '品牌管理', icon: 'pi pi-palette', path: '/main/dev-brand' },
      { label: '版本管理', icon: 'pi pi-tag', path: '/main/dev-version' },
    ],
  }] as MenuItem[] : []),
]

const expandedGroup = ref<string | null>(null)

const expandedItem = computed(() =>
  menuItems.find(m => m.label === expandedGroup.value && m.children)
)

const pageTitle = computed(() => {
  for (const item of menuItems) {
    if (item.path && route.path === item.path) return item.label
    if (item.children) {
      const sub = item.children.find(c => route.path === c.path)
      if (sub) return sub.label
    }
  }
  return '主页'
})

function isItemActive(item: MenuItem): boolean {
  if (item.path) return route.path === item.path
  return !!item.children?.some(c => route.path === c.path)
}

const showUserMenu = ref(false)
const _router = useRouter()

function toggleUserMenu(e: MouseEvent) {
  e.stopPropagation()
  showUserMenu.value = !showUserMenu.value
}

function closeUserMenu() {
  showUserMenu.value = false
}

function handleMenuClick(item: MenuItem) {
  if (item.children) {
    expandedGroup.value = expandedGroup.value === item.label ? null : item.label
  } else if (item.path) {
    expandedGroup.value = null
    _router.push(item.path)
  }
}

function handleSubClick(sub: SubItem) {
  _router.push(sub.path)
}

function handleEditProfile() {
  showUserMenu.value = false
  openProfileModal()
}

function handleRecharge() {
  showUserMenu.value = false
  rechargeCardKey.value = ''
  rechargeErrMsg.value = ''
  rechargeSuccessMsg.value = ''
  rechargeTab.value = hasOnlinePay.value ? 'online' : 'card'
  showRechargeModal.value = true
  if (hasOnlinePay.value && packages.value.length === 0) loadPackages()
}

function handleChangePassword() {
  showUserMenu.value = false
  openChangePwdModal()
}

function handleUnbindDevice() {
  showUserMenu.value = false
  openUnbindModal()
}

async function onLogout() {
  showUserMenu.value = false
  clearCheckin()
  await handleLogout()
}

onMounted(() => {
  initHeartbeat()
  if (!import.meta.env.DEV) checkForUpdate()
  document.addEventListener('click', closeUserMenu)

  const info = userStore.userInfo
  if (info && (!info.acctno || !info.email)) {
    setTimeout(() => handleEditProfile(), 500)
  }
})

onUnmounted(() => {
  stopPayPolling()
  destroyHeartbeat()
  document.removeEventListener('click', closeUserMenu)
})
</script>

<template>
  <div class="layout-root">
    <TitleBar variant="full" :title="pageTitle" :contact-float-visible="showContactFloat" @restore-contact="restoreContactFloat" />

    <div class="layout-body">
      <!-- Icon Rail -->
      <div class="icon-rail">
        <div class="ir-logo">
          <img :src="brandLogo" class="ir-logo-img" alt="" />
        </div>
        <nav class="ir-nav">
          <button v-for="item in menuItems" :key="item.label" type="button" class="ir-btn" :class="{ active: isItemActive(item), expanded: expandedGroup === item.label }" :title="item.label" @click="handleMenuClick(item)">
            <i :class="item.icon"></i>
            <span v-if="item.children" class="ir-badge">{{ item.children.length }}</span>
          </button>
        </nav>
        <div class="ir-bottom">
          <div class="ir-user" @click="toggleUserMenu($event)">
            <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="ir-user-img" alt="" />
            <span v-else class="ir-user-text">{{ displayName.charAt(0) }}</span>
          </div>
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
              <button class="ud-item" @click="handleEditProfile"><i class="pi pi-user-edit"></i> 编辑资料</button>
              <button class="ud-item" @click="handleRecharge"><i class="pi pi-credit-card"></i> 卡密充值</button>
              <button class="ud-item" @click="handleChangePassword"><i class="pi pi-lock"></i> 修改密码</button>
              <button class="ud-item" @click="handleUnbindDevice"><i class="pi pi-link"></i> 解绑设备</button>
              <div class="ud-divider"></div>
              <button class="ud-item ud-item--danger" @click="onLogout"><i class="pi pi-sign-out"></i> 退出登录</button>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Sub Panel (二级面板) -->
      <Transition name="sp-slide">
        <div v-if="expandedItem" class="sub-panel" :key="expandedItem.label">
          <div class="sp-header">{{ expandedItem.label }}</div>
          <nav class="sp-nav">
            <button v-for="sub in expandedItem.children" :key="sub.path" type="button" class="sp-item" :class="{ active: route.path === sub.path }" @click="handleSubClick(sub)">
              <i v-if="sub.icon" :class="sub.icon" class="sp-icon"></i>
              <span>{{ sub.label }}</span>
            </button>
          </nav>
        </div>
      </Transition>

      <div class="layout-main">
        <header class="main-header">
          <div class="header-left">
            <span class="header-brand">{{ brand.brand_name }}</span>
            <span class="header-sep">/</span>
            <span class="header-page">{{ pageTitle }}</span>
          </div>
          <div class="ch-actions">
            <button v-if="brand.tutorial_url" class="ch-tutorial-btn" @click="openUrl(brand.tutorial_url)">
              <i class="pi pi-book"></i> 使用教程
            </button>
            <div v-if="brand.checkin_reward_value > 0" class="ch-checkin-wrap">
              <button class="ch-checkin-btn" :class="{ 'ch-checkin-done': !canCheckin }" :disabled="checkinLoading || !canCheckin" @click="handleCheckin">
                <i class="pi" :class="canCheckin ? 'pi-sun' : 'pi-check-circle'"></i>
                {{ checkinLoading ? '签到中…' : canCheckin ? '每日签到' : '已签到' }}
              </button>
              <Transition name="fade">
                <div v-if="showCheckinHint" class="ch-checkin-badge">
                  <i class="pi pi-gift"></i> 签到领 {{ rewardSummary }}
                </div>
              </Transition>
              <Transition name="fade">
                <div v-if="checkinMsg" class="ch-checkin-toast">{{ checkinMsg }}</div>
              </Transition>
            </div>
            <button class="ch-recharge-btn" @click="handleRecharge">
              <i class="pi pi-bolt"></i>
              <span>充值中心</span>
            </button>
          </div>
        </header>
        <main class="content-area">
          <RouterView />
        </main>

      </div>
    </div>

    <!-- 充值中心弹窗 -->
    <Transition name="modal">
      <div v-if="showRechargeModal" class="modal-overlay" @click.self="closeRechargeModal">
        <div class="modal-box modal-box--recharge">
          <div class="modal-header">
            <h3><i class="pi pi-bolt rc-title-icon"></i>充值中心</h3>
            <button type="button" class="modal-close" @click="closeRechargeModal">
              <i class="pi pi-times"></i>
            </button>
          </div>

          <div v-if="hasOnlinePay" class="rc-tabs">
            <button type="button" class="rc-tab" :class="{ active: rechargeTab === 'online' }" @click="rechargeTab = 'online'">
              <i class="pi pi-shopping-cart"></i> 在线支付
            </button>
            <button type="button" class="rc-tab" :class="{ active: rechargeTab === 'card' }" @click="rechargeTab = 'card'">
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
                  v-for="pkg in packages"
                  :key="pkg.id"
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
                    <input v-model="selectedPayMethod" type="radio" value="wechat" />
                    <i class="pi pi-microsoft rc-pay-icon--wx"></i>
                    <span>微信支付</span>
                  </label>
                  <label v-if="hasAlipay" class="rc-pay-opt" :class="{ active: selectedPayMethod === 'alipay' }">
                    <input v-model="selectedPayMethod" type="radio" value="alipay" />
                    <i class="pi pi-wallet rc-pay-icon--ali"></i>
                    <span>支付宝</span>
                  </label>
                </div>
              </div>

              <Transition name="fade">
                <div v-if="rechargeErrMsg" class="modal-msg modal-msg--err">
                  <i class="pi pi-exclamation-circle"></i>{{ rechargeErrMsg }}
                </div>
              </Transition>

              <button type="button" class="rc-pay-btn" :disabled="!selectedPkg || rechargeLoading" @click="startPay">
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

    <!-- 支付二维码弹窗 -->
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

    <!-- Profile modal -->
    <Transition name="modal">
      <div v-if="showProfileModal" class="modal-overlay" @click.self="closeProfileModal">
        <div class="modal-box modal-box--wide">
          <div class="modal-header">
            <h3>编辑资料</h3>
            <button type="button" class="modal-close" @click="closeProfileModal"><i class="pi pi-times"></i></button>
          </div>
          <form class="modal-body" @submit.prevent="submitProfile">
            <div class="profile-avatar-section">
              <div class="profile-avatar-wrap" @click="triggerAvatarSelect">
                <img v-if="profileAvatarPreview" :src="profileAvatarPreview" class="profile-avatar-img" alt="" />
                <div v-else class="profile-avatar-placeholder">{{ displayName.charAt(0) }}</div>
                <div class="profile-avatar-hover"><i class="pi pi-camera"></i></div>
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
                  <Button type="button" :label="emailBtnText" :disabled="emailCooldown > 0" :loading="emailSending" class="email-code-btn" @click="handleSendEmailCode" />
                </div>
              </template>
            </div>
            <Transition name="fade">
              <div v-if="profileErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ profileErrMsg }}</div>
              <div v-else-if="profileSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ profileSuccessMsg }}</div>
            </Transition>
            <Button type="submit" label="保存" :loading="profileLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- Change password modal -->
    <Transition name="modal">
      <div v-if="showChangePwdModal" class="modal-overlay" @click.self="showChangePwdModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>修改密码</h3>
            <button type="button" class="modal-close" @click="showChangePwdModal = false"><i class="pi pi-times"></i></button>
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
              <div v-if="cpErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ cpErrMsg }}</div>
              <div v-else-if="cpSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ cpSuccessMsg }}</div>
            </Transition>
            <Button type="submit" label="确认修改" :loading="cpLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- Unbind device modal -->
    <Transition name="modal">
      <div v-if="showUnbindModal" class="modal-overlay" @click.self="showUnbindModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>解绑当前设备</h3>
            <button type="button" class="modal-close" @click="showUnbindModal = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="modal-body" style="text-align:center">
            <p style="font-size:0.88rem;color:#606266;margin:0 0 20px;line-height:1.6">{{ getUnbindTip() }}</p>
            <Transition name="fade">
              <div v-if="unbindErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ unbindErrMsg }}</div>
              <div v-else-if="unbindSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ unbindSuccessMsg }}</div>
            </Transition>
            <Button label="确认解绑" :loading="unbindLoading" class="modal-submit" style="margin-top:8px" @click="submitUnbind" />
          </div>
        </div>
      </div>
    </Transition>

    <!-- Expired modal -->
    <Transition name="modal">
      <div v-if="showExpired" class="modal-overlay">
        <div class="modal-box">
          <div class="modal-header"><h3>使用到期</h3></div>
          <div class="modal-body" style="text-align:center">
            <div class="expired-icon"><i class="pi pi-clock"></i></div>
            <p class="expired-msg">{{ expiredMsg }}</p>
            <div style="display:flex;gap:10px">
              <Button label="卡密充值" class="modal-submit" style="flex:1" @click="showExpired = false; handleRecharge()" />
              <Button label="退出登录" class="modal-submit modal-submit--secondary" style="flex:1" @click="forceExpired = false; showExpired = false; onLogout()" />
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Banned overlay -->
    <div v-if="showBanned" class="banned-overlay">
      <div class="banned-box">
        <div class="banned-icon"><i class="pi pi-ban"></i></div>
        <h2 class="banned-title">{{ bannedTitle }}</h2>
        <p class="banned-msg">{{ bannedMsg }}</p>
        <p class="banned-countdown">{{ bannedCountdown }} 秒后自动退出登录</p>
      </div>
    </div>

    <!-- Floating Contact Button -->
    <Transition name="contact-float">
      <button v-if="showContactFloat && hasContactImages" class="contact-float" :class="{ 'contact-highlight': contactHighlight }" @click="showContactModal = true">
        <i class="pi pi-comments"></i>
        <span>联系我们</span>
        <i class="pi pi-times contact-float-close" @click.stop="closeContactFloat"></i>
      </button>
    </Transition>

    <!-- Contact Images Modal -->
    <Transition name="modal">
      <div v-if="showContactModal" class="modal-overlay" @click.self="showContactModal = false">
        <div class="modal-box modal-box--contact">
          <div class="modal-header">
            <h3>联系我们</h3>
            <button type="button" class="modal-close" @click="showContactModal = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="modal-body contact-images">
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
  </div>
</template>

<style scoped>
@keyframes fade-in { from { opacity: 0 } to { opacity: 1 } }

.layout-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #F5F7FA;
  animation: fade-in 0.3s ease-out;
}

.layout-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* ── Icon Rail — modern, integrated, no admin feel ── */
.icon-rail {
  width: 56px;
  flex-shrink: 0;
  background: #fff;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 0;
  z-index: 100;
}

.ir-logo {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  overflow: hidden;
  margin-bottom: 20px;
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(249, 115, 22, 0.15);
}

.ir-logo-img { width: 100%; height: 100%; object-fit: contain; }

.ir-nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.ir-btn {
  width: 38px;
  height: 38px;
  border: none;
  border-radius: 12px;
  background: transparent;
  color: #C0C4CC;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
  position: relative;
}

.ir-btn i { font-size: 1.05rem; }

.ir-btn:hover {
  background: #FFF7ED;
  color: #F97316;
}

.ir-btn.active {
  background: linear-gradient(135deg, #F97316, #EA580C);
  color: #fff;
  box-shadow: 0 3px 10px rgba(249, 115, 22, 0.3);
}

.ir-bottom { margin-top: auto; }

.ir-badge {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #F97316;
  color: #fff;
  font-size: 0.55rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.ir-btn.active .ir-badge,
.ir-btn.expanded .ir-badge { background: #fff; color: #F97316; }

/* ── Sub Panel ── */
.sub-panel {
  width: 170px;
  flex-shrink: 0;
  background: #fff;
  border-right: 1px solid #F0F0F0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sp-header {
  padding: 16px 16px 10px;
  font-size: 0.78rem;
  font-weight: 700;
  color: #303133;
  letter-spacing: 0.02em;
}

.sp-nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0 8px;
}

.sp-item {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #909399;
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.sp-icon { font-size: 0.82rem; width: 16px; text-align: center; }

.sp-item:hover { background: #FFF7ED; color: #F97316; }

.sp-item.active {
  background: #FFF7ED;
  color: #F97316;
  font-weight: 600;
}

.sp-slide-enter-active { transition: width 0.25s ease, opacity 0.2s ease; }
.sp-slide-leave-active { transition: width 0.2s ease, opacity 0.15s ease; }
.sp-slide-enter-from,
.sp-slide-leave-to { width: 0; opacity: 0; }

.ir-user {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  overflow: hidden;
  cursor: pointer;
  border: 2px solid #EBEEF5;
  transition: border-color 0.2s;
  background: linear-gradient(135deg, #F97316, #EA580C);
  display: flex;
  align-items: center;
  justify-content: center;
}

.ir-user:hover { border-color: #F97316; }
.ir-user-img { width: 100%; height: 100%; object-fit: cover; }
.ir-user-text { color: #fff; font-weight: 700; font-size: 0.75rem; }

/* ── Main area ── */
.layout-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

/* ── Header ── */
.main-header {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  background: #fff;
  flex-shrink: 0;
}

.header-left { display: flex; align-items: center; gap: 6px; }

.ch-tutorial-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 14px;
  border: 1px solid var(--app-primary-border, #FDBA74);
  border-radius: 6px;
  background: var(--app-primary-light, #FFF7ED);
  color: var(--app-primary, #F97316);
  font-size: 0.78rem;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}
.ch-tutorial-btn:hover {
  background: var(--app-primary-light-hover, #FFEDD5);
  border-color: var(--app-primary, #F97316);
}
.ch-actions { display: flex; align-items: center; gap: 8px; }
.ch-checkin-wrap { position: relative; display: inline-flex; align-items: center; }
.ch-checkin-btn {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 5px 14px; border: 1px solid var(--app-primary, #F97316); border-radius: 6px;
  background: var(--app-primary, #F97316); color: #fff;
  font-size: 0.78rem; font-family: inherit; font-weight: 600;
  cursor: pointer; transition: all 0.15s;
}
.ch-checkin-btn:hover:not(:disabled) { background: var(--app-primary-hover, #EA580C); }
.ch-checkin-btn:disabled { opacity: 0.65; cursor: default; }
.ch-checkin-done {
  background: var(--app-primary-light, #FFF7ED) !important;
  color: var(--app-primary, #F97316) !important;
  border-color: var(--app-primary-border, #FDBA74) !important;
}
.ch-checkin-badge {
  position: absolute; top: calc(100% + 6px); left: 50%; transform: translateX(-50%);
  white-space: nowrap; padding: 4px 10px;
  background: #FFF7ED; color: #EA580C; border: 1px solid #FDBA74;
  font-size: 0.68rem; font-weight: 500; border-radius: 4px;
  box-shadow: 0 2px 8px rgba(249,115,22,0.15); z-index: 10; pointer-events: none;
}
.ch-checkin-badge .pi { font-size: 0.68rem; }
.ch-checkin-toast {
  position: absolute; top: calc(100% + 6px); left: 50%; transform: translateX(-50%);
  white-space: nowrap; padding: 4px 10px;
  background: #303133; color: #fff;
  font-size: 0.7rem; border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.2); z-index: 10; pointer-events: none;
}

.ch-recharge-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 16px;
  border: none;
  border-radius: 6px;
  background: linear-gradient(135deg, #F97316, #EA580C);
  color: #fff;
  font-size: 0.78rem;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  box-shadow: 0 2px 10px rgba(249, 115, 22, 0.35);
  transition: all 0.2s;
  letter-spacing: 0.02em;
}
.ch-recharge-btn:hover {
  background: linear-gradient(135deg, #EA580C, #C2410C);
  box-shadow: 0 4px 14px rgba(234, 88, 12, 0.45);
  transform: translateY(-1px);
}
.ch-recharge-btn .pi { font-size: 0.82rem; }

.header-brand { font-size: 0.82rem; color: #C0C4CC; font-weight: 500; }
.header-sep { font-size: 0.82rem; color: #DCDFE6; }
.header-page { font-size: 0.82rem; font-weight: 600; color: #303133; }

.avatar-img { width: 100%; height: 100%; object-fit: cover; }

/* ── User Dropdown (opens upward from avatar) ── */
.ir-bottom { position: relative; }

.user-dropdown {
  position: absolute;
  bottom: calc(100% + 8px);
  left: calc(100% + 8px);
  min-width: 200px;
  background: #fff;
  border: 1px solid #EBEEF5;
  border-radius: 8px;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.1);
  padding: 4px 0;
  z-index: 500;
}

.ud-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px 10px;
}

.ud-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: linear-gradient(135deg, #F97316, #EA580C);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.9rem;
  flex-shrink: 0;
  overflow: hidden;
}

.ud-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.ud-info strong { font-size: 0.88rem; color: #303133; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ud-info span { font-size: 0.72rem; color: #909399; }
.ud-divider { height: 1px; background: #EBEEF5; margin: 4px 0; }

.ud-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 9px 16px;
  border: none;
  background: none;
  color: #606266;
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.12s;
  white-space: nowrap;
}

.ud-item:hover { background: #F5F7FA; color: #303133; }
.ud-item i { font-size: 0.88rem; width: 16px; text-align: center; color: #909399; }
.ud-item:hover i { color: #F97316; }
.ud-item--danger { color: #F56C6C; }
.ud-item--danger i { color: #F56C6C; }
.ud-item--danger:hover { background: #FEF0F0; color: #F56C6C; }
.ud-item--danger:hover i { color: #F56C6C; }

.dropdown-enter-active, .dropdown-leave-active { transition: all 0.2s ease; }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(8px); }

/* ── Content Area ── */
.content-area {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 20px;
}

/* ── Modals (Geeker-Admin style) ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
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
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.12);
  overflow: hidden;
}

.modal-box--wide { width: 440px; }
.modal-box--recharge { width: 480px; }

.rc-title-icon {
  color: var(--app-primary, #F97316);
  margin-right: 6px;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #EBEEF5;
}

.modal-header h3 { margin: 0; font-size: 1rem; font-weight: 600; color: #303133; }

.modal-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: #909399;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.modal-close:hover { background: #F5F7FA; color: #303133; }

.modal-body { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
.modal-field { display: flex; flex-direction: column; gap: 6px; }
.modal-field label { font-size: 0.82rem; font-weight: 500; color: #606266; }

.modal-readonly {
  height: 40px;
  line-height: 40px;
  font-size: 0.88rem;
  color: #303133;
  font-weight: 500;
  padding: 0 12px;
  background: #F5F7FA;
  border-radius: 4px;
  border: 1px solid #EBEEF5;
}

.modal-acctno {
  height: 40px;
  line-height: 40px;
  font-size: 0.88rem;
  color: #303133;
  font-weight: 600;
  padding: 0 12px;
  background: #F5F7FA;
  border-radius: 4px;
  border: 1px solid #EBEEF5;
}

.modal-field :deep(.modal-input) {
  width: 100%;
  height: 40px;
  font-size: 0.88rem;
  border: 1px solid #DCDFE6;
  border-radius: 4px;
  background: #fff;
  padding: 0 12px;
  transition: all 0.2s;
  color: #303133;
}

.modal-field :deep(.modal-input:focus) {
  border-color: #F97316;
  box-shadow: 0 0 0 2px rgba(249, 115, 22, 0.1);
}

.modal-msg {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.78rem;
  padding: 8px 12px;
  border-radius: 4px;
}

.modal-msg--err { color: #F56C6C; background: #FEF0F0; border: 1px solid #FDE2E2; }
.modal-msg--ok { color: #67C23A; background: #F0F9EB; border: 1px solid #E1F3D8; }

.modal-submit {
  width: 100%;
  height: 40px;
  font-size: 0.92rem;
  font-weight: 600;
  border-radius: 4px;
  background: #F97316 !important;
  border: none !important;
  transition: all 0.2s;
  margin-top: 4px;
}

.modal-submit:hover { background: #EA580C !important; }

.modal-submit--secondary {
  background: #909399 !important;
}

.modal-submit--secondary:hover { background: #606266 !important; }

.modal-enter-active, .modal-leave-active { transition: opacity 0.2s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-active .modal-box, .modal-leave-active .modal-box { transition: transform 0.2s ease; }
.modal-enter-from .modal-box { transform: scale(0.96); }
.modal-leave-to .modal-box { transform: scale(0.96); }

.fade-enter-active, .fade-leave-active { transition: all 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }

/* ── Recharge center (tabs, packages, QR) ── */
.rc-tabs {
  display: flex;
  border-bottom: 2px solid #EBEEF5;
  padding: 0 20px;
}
.rc-tab {
  flex: 1;
  padding: 12px 0;
  border: none;
  background: none;
  font-size: 0.86rem;
  font-weight: 600;
  font-family: inherit;
  color: #909399;
  cursor: pointer;
  position: relative;
  transition: color 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
.rc-tab.active { color: var(--app-primary, #F97316); }
.rc-tab.active::after {
  content: '';
  position: absolute;
  bottom: -2px;
  left: 20%;
  right: 20%;
  height: 2px;
  background: var(--app-primary, #F97316);
  border-radius: 1px;
}

.rc-body { padding: 20px; }
.rc-loading, .rc-empty {
  text-align: center;
  padding: 28px 0;
  color: #909399;
  font-size: 0.84rem;
}

.rc-packages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 10px;
  margin-bottom: 16px;
}
.rc-pkg-card {
  padding: 12px 10px;
  border: 2px solid #EBEEF5;
  border-radius: 8px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: #fff;
}
.rc-pkg-card:hover {
  border-color: var(--app-primary-border, #FDBA74);
  background: var(--app-primary-light, #FFF7ED);
}
.rc-pkg-card.selected {
  border-color: var(--app-primary, #F97316);
  background: linear-gradient(135deg, #FFF7ED, #FFEDD5);
  box-shadow: 0 0 0 1px var(--app-primary, #F97316);
}
.rc-pkg-name {
  font-size: 0.8rem;
  font-weight: 700;
  color: #303133;
  margin-bottom: 4px;
}
.rc-pkg-desc {
  font-size: 0.7rem;
  color: #909399;
  margin-bottom: 8px;
}
.rc-pkg-price { color: #F56C6C; font-weight: 700; }
.rc-price-symbol { font-size: 0.72rem; }
.rc-price-value { font-size: 1.15rem; }

.rc-pay-methods {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 12px;
  background: #F5F7FA;
  border-radius: 8px;
}
.rc-pay-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: #606266;
  white-space: nowrap;
}
.rc-pay-options { display: flex; gap: 10px; flex-wrap: wrap; }
.rc-pay-opt {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid #DCDFE6;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.76rem;
  color: #606266;
  transition: all 0.15s;
  background: #fff;
}
.rc-pay-opt input { display: none; }
.rc-pay-opt.active {
  border-color: var(--app-primary, #F97316);
  background: var(--app-primary-light, #FFF7ED);
  color: var(--app-primary, #F97316);
  font-weight: 600;
}
.rc-pay-icon--wx { color: #07c160; }
.rc-pay-icon--ali { color: #1677ff; }

.rc-pay-btn {
  width: 100%;
  padding: 11px;
  border: none;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--app-primary, #F97316), var(--app-primary-hover, #EA580C));
  color: #fff;
  font-size: 0.92rem;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(249, 115, 22, 0.3);
}
.rc-pay-btn:hover:not(:disabled) {
  box-shadow: 0 6px 18px rgba(249, 115, 22, 0.4);
  transform: translateY(-1px);
}
.rc-pay-btn:disabled { opacity: 0.6; cursor: default; }

.qr-modal { width: 360px; }
.qr-body { padding: 20px; text-align: center; }
.qr-wrap { display: flex; flex-direction: column; align-items: center; gap: 12px; }
.qr-img {
  width: 200px;
  height: 200px;
  border: 1px solid #EBEEF5;
  border-radius: 8px;
  padding: 8px;
}
.qr-hint { font-size: 0.8rem; color: #909399; margin: 0; }
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
.qr-amount { font-size: 1.35rem; font-weight: 800; color: #F56C6C; margin: 0; }
.qr-polling {
  font-size: 0.76rem;
  color: #909399;
  display: flex;
  align-items: center;
  gap: 6px;
}
.qr-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 20px 0;
}
.qr-result .pi { font-size: 2.75rem; }
.qr-result p { font-size: 0.95rem; font-weight: 600; margin: 0; }
.qr-result--ok { color: var(--app-primary, #F97316); }
.qr-result--fail { color: #F56C6C; }

/* ── Profile modal ── */
.profile-avatar-section { display: flex; flex-direction: column; align-items: center; gap: 6px; padding-bottom: 8px; }

.profile-avatar-wrap {
  position: relative;
  width: 72px;
  height: 72px;
  border-radius: 50%;
  cursor: pointer;
  overflow: hidden;
  border: 2px solid #EBEEF5;
}

.profile-avatar-img { width: 100%; height: 100%; object-fit: cover; }

.profile-avatar-placeholder {
  width: 100%;
  height: 100%;
  background: linear-gradient(135deg, #F97316, #EA580C);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.8rem;
  font-weight: 700;
}

.profile-avatar-hover {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s;
}

.profile-avatar-hover i { color: #fff; font-size: 1.2rem; }
.profile-avatar-wrap:hover .profile-avatar-hover { opacity: 1; }
.profile-avatar-hint { font-size: 0.72rem; color: #909399; }

/* ── Banned overlay ── */
.banned-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.banned-box {
  text-align: center;
  padding: 40px 48px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
  max-width: 400px;
}

.banned-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: #FEF0F0;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
}

.banned-icon i { font-size: 1.8rem; color: #F56C6C; }
.banned-title { margin: 0 0 10px; font-size: 1.2rem; font-weight: 600; color: #F56C6C; }
.banned-msg { margin: 0 0 16px; font-size: 0.88rem; color: #606266; line-height: 1.6; }
.banned-countdown { margin: 0; font-size: 0.82rem; color: #909399; }

.expired-icon {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: #FFF7ED;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 14px;
}

.expired-icon i { font-size: 1.5rem; color: #F97316; }
.expired-msg { font-size: 0.88rem; color: #606266; line-height: 1.6; margin: 0 0 20px; }

.field-hint { font-weight: 400; font-size: 0.7rem; color: #909399; margin-left: 4px; }

.email-bound {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 12px;
  background: #F5F7FA;
  border-radius: 4px;
  border: 1px solid #EBEEF5;
  font-size: 0.88rem;
  color: #303133;
}

.email-change-btn {
  border: none;
  background: none;
  color: #F97316;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.15s;
}

.email-change-btn:hover { background: #FFF7ED; }

.email-code-row { display: flex; gap: 8px; align-items: center; margin-top: 8px; }
.email-code-row :deep(.email-code-input) { flex: 1; min-width: 0; }

.email-code-btn {
  flex-shrink: 0;
  height: 40px;
  font-size: 0.78rem;
  font-weight: 600;
  border-radius: 4px;
  white-space: nowrap;
  padding: 0 14px;
  background: #F97316 !important;
  border: none !important;
  color: #fff !important;
  transition: all 0.2s;
}

.email-code-btn:hover { background: #EA580C !important; }
.email-code-btn:disabled { opacity: 0.6; }

/* ─── Floating Contact Button ─── */
.contact-float {
  position: fixed; bottom: 28px; right: 28px; z-index: 900;
  display: flex; align-items: center; gap: 6px;
  padding: 10px 18px; border: 1px solid var(--app-primary-border, #FDBA74); border-radius: 28px;
  background: #fff; color: var(--app-primary, #F97316);
  font-size: 0.82rem; font-family: inherit; font-weight: 500;
  cursor: pointer; transition: all 0.2s;
  box-shadow: 0 4px 16px rgba(0,0,0,0.08);
}
.contact-float:hover { background: var(--app-primary-light, #FFF7ED); border-color: var(--app-primary); box-shadow: 0 4px 20px rgba(249,115,22,0.15); }
.contact-float-close { margin-left: 4px; font-size: 0.7rem; opacity: 0.4; transition: opacity 0.15s; }
.contact-float-close:hover { opacity: 1; }
.contact-highlight { animation: contact-pulse 0.6s ease-in-out 3; }
@keyframes contact-pulse {
  0%, 100% { box-shadow: 0 4px 16px rgba(0,0,0,0.08); }
  50% { box-shadow: 0 0 0 8px rgba(249,115,22,0.15), 0 4px 16px rgba(249,115,22,0.1); }
}
.contact-float-enter-active, .contact-float-leave-active { transition: all 0.3s ease; }
.contact-float-enter-from, .contact-float-leave-to { opacity: 0; transform: translateY(20px) scale(0.9); }

.modal-box--contact { max-width: 520px; }
.contact-images { display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; padding: 8px 0; }
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
