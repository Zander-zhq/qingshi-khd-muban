<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { redeemCardApi } from '../../../api/auth'
import { getAppCredentials } from '../../../utils/config'
import { useGuestPay } from '../../../composables/useGuestPay'

const router = useRouter()
const appId = ref('')

const {
  hasOnlinePay, hasWechat, hasAlipay,
  packages, packagesLoading, selectedPkg, selectedPayMethod,
  payQrUrl, showQrModal, payStatus,
  payLoading, payErrMsg, paySuccessMsg,
  loadPackages, formatPrice, formatDuration, selectPackage,
  startPay, closeQrModal, resetPayState,
} = useGuestPay()

const activeTab = ref<'online' | 'card'>('card')

onMounted(async () => {
  const creds = await getAppCredentials()
  appId.value = creds.appId
  if (hasOnlinePay.value) {
    activeTab.value = 'online'
    loadPackages()
  }
})

const acctno = ref('')
const cardKey = ref('')
const loading = ref(false)
const errMsg = ref('')
const successMsg = ref('')

function clearMsg() {
  errMsg.value = ''
  successMsg.value = ''
}

function switchTab(tab: 'online' | 'card') {
  activeTab.value = tab
  clearMsg()
  resetPayState()
}

async function handleRecharge() {
  clearMsg()

  if (!acctno.value.trim()) { errMsg.value = '请输入账号（手机号）'; return }
  if (!cardKey.value.trim()) { errMsg.value = '请输入卡密'; return }

  loading.value = true
  try {
    const res = await redeemCardApi({
      app_id: appId.value,
      acctno: acctno.value.trim(),
      card_key: cardKey.value.trim(),
    })
    const cardType = (res as any).card_type || ''
    const expireAt = (res as any).vip_expire_at || ''
    let msg = '充值成功！'
    if (cardType) msg += ` (${cardType})`
    if (expireAt) {
      const d = new Date(expireAt)
      msg += `  到期：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    }
    successMsg.value = msg
    cardKey.value = ''
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '充值失败'
  } finally {
    loading.value = false
  }
}

function handleStartPay() {
  if (!acctno.value.trim()) {
    payErrMsg.value = '请输入账号（手机号）'
    return
  }
  startPay(acctno.value)
}
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="banner-title">充值中心</div>
      </div>

      <div class="body">
        <div class="form-area">
          <!-- 账号输入（共用） -->
          <div class="acctno-box">
            <InputText
              v-model="acctno"
              placeholder="账号（手机号）"
              class="field-input"
              @input="clearMsg"
            />
          </div>

          <!-- Tab 切换 -->
          <div v-if="hasOnlinePay" class="rc-tabs">
            <button class="rc-tab" :class="{ active: activeTab === 'online' }" @click="switchTab('online')">
              <i class="pi pi-shopping-cart"></i> 在线支付
            </button>
            <button class="rc-tab" :class="{ active: activeTab === 'card' }" @click="switchTab('card')">
              <i class="pi pi-credit-card"></i> 卡密充值
            </button>
          </div>

          <!-- 在线支付 -->
          <div v-if="activeTab === 'online' && hasOnlinePay" class="rc-online">
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
                <div v-if="payErrMsg" class="msg-tip msg-err">
                  <i class="pi pi-exclamation-circle"></i> {{ payErrMsg }}
                </div>
                <div v-else-if="paySuccessMsg" class="msg-tip msg-ok">
                  <i class="pi pi-check-circle"></i> {{ paySuccessMsg }}
                </div>
              </Transition>

              <button class="submit-btn pay-btn" :disabled="!selectedPkg || payLoading" @click="handleStartPay">
                <i v-if="payLoading" class="pi pi-spin pi-spinner"></i>
                <template v-else>
                  <i class="pi pi-bolt"></i>
                  立即支付 {{ selectedPkg ? '¥' + formatPrice(selectedPkg.price) : '' }}
                </template>
              </button>
            </template>
          </div>

          <!-- 卡密充值 -->
          <form v-if="activeTab === 'card' || !hasOnlinePay" class="form" @submit.prevent="handleRecharge">
            <div class="field-box">
              <InputText
                v-model="cardKey"
                placeholder="卡密"
                class="field-input"
                @input="clearMsg"
              />
            </div>

            <Transition name="fade">
              <div v-if="errMsg" class="msg-tip msg-err">
                <i class="pi pi-exclamation-circle"></i>
                {{ errMsg }}
              </div>
              <div v-else-if="successMsg" class="msg-tip msg-ok">
                <i class="pi pi-check-circle"></i>
                {{ successMsg }}
              </div>
            </Transition>

            <Button type="submit" label="充 值" :loading="loading" class="submit-btn" />
          </form>
        </div>

        <div class="bottom-links">
          <a href="#" class="link-text" @click.prevent="router.push('/login')">
            <i class="pi pi-arrow-left" style="font-size: 0.7rem"></i>
            返回登录
          </a>
        </div>
      </div>

      <!-- 支付二维码弹窗 -->
      <Transition name="modal">
        <div v-if="showQrModal" class="qr-overlay" @click.self="closeQrModal">
          <div class="qr-modal">
            <div class="qr-header">
              <h3>{{ selectedPayMethod === 'wechat' ? '微信支付' : '支付宝支付' }}</h3>
              <button type="button" class="qr-close" @click="closeQrModal">
                <i class="pi pi-times"></i>
              </button>
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
                <a href="#" class="link-text" @click.prevent="closeQrModal(); router.push('/login')">返回登录</a>
              </div>
              <div v-else class="qr-result qr-result--fail">
                <i class="pi pi-times-circle"></i>
                <p>支付失败，请重试</p>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.window-shell {
  height: 100vh;
  width: 100vw;
  background: #fff;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
  overflow-y: auto;
}

.window-content :deep(.app-titlebar.titlebar-compact) {
  position: relative;
}

.banner {
  height: 100px;
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
.bc-1 { width: 140px; height: 140px; top: -50px; right: -20px; }
.bc-2 { width: 80px; height: 80px; bottom: -30px; left: 10px; }

.banner-title {
  position: relative;
  z-index: 1;
  font-size: 1.4rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.12em;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.1);
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 0 36px;
  min-height: 0;
}

.form-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 20px;
}

.acctno-box {
  width: 100%;
  margin-bottom: 12px;
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

.field-box :deep(.field-input),
.acctno-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%;
  height: 44px;
  font-size: 0.92rem;
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
.acctno-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: var(--qs-primary-light);
  background: #fff;
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.08);
}

/* ── Tab 切换 ── */
.rc-tabs {
  width: 100%;
  display: flex;
  gap: 0;
  margin-bottom: 16px;
  border-radius: 10px;
  overflow: hidden;
  border: 1.5px solid #e2e8f0;
}

.rc-tab {
  flex: 1;
  padding: 10px 0;
  font-size: 0.88rem;
  font-weight: 500;
  border: none;
  background: #f8fafb;
  color: #64748b;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.2s;
}

.rc-tab.active {
  background: var(--qs-bg-gradient);
  color: #fff;
  font-weight: 600;
}

.rc-tab:not(.active):hover {
  background: #f0f4f8;
}

/* ── 在线支付区域 ── */
.rc-online {
  width: 100%;
}

.rc-loading,
.rc-empty {
  text-align: center;
  padding: 24px 0;
  color: #94a3b8;
  font-size: 0.88rem;
}

.rc-packages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 10px;
  margin-bottom: 14px;
}

.rc-pkg-card {
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  padding: 12px 10px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: #f8fafb;
}

.rc-pkg-card:hover {
  border-color: var(--qs-primary-light);
  background: #f0fdfa;
}

.rc-pkg-card.selected {
  border-color: var(--qs-primary);
  background: #f0fdfa;
  box-shadow: 0 0 0 2px rgba(13, 148, 136, 0.15);
}

.rc-pkg-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: #1e293b;
  margin-bottom: 4px;
}

.rc-pkg-desc {
  font-size: 0.75rem;
  color: #94a3b8;
  margin-bottom: 6px;
}

.rc-pkg-price {
  color: var(--qs-primary);
  font-weight: 700;
}

.rc-price-symbol {
  font-size: 0.75rem;
}

.rc-price-value {
  font-size: 1.1rem;
}

/* ── 支付方式 ── */
.rc-pay-methods {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}

.rc-pay-label {
  font-size: 0.82rem;
  color: #64748b;
  white-space: nowrap;
}

.rc-pay-options {
  display: flex;
  gap: 10px;
}

.rc-pay-opt {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.82rem;
  transition: all 0.2s;
  background: #f8fafb;
}

.rc-pay-opt input[type="radio"] { display: none; }

.rc-pay-opt.active {
  border-color: var(--qs-primary);
  background: #f0fdfa;
}

.rc-pay-opt:hover {
  border-color: var(--qs-primary-light);
}

.msg-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  padding: 8px 12px;
  border-radius: 8px;
}

.msg-err {
  color: #dc2626;
  background: #fef2f2;
  border: 1px solid #fecaca;
}

.msg-ok {
  color: var(--qs-primary-dark);
  background: #f0fdfa;
  border: 1px solid #99f6e4;
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

.submit-btn {
  width: 100%;
  height: 44px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 22px;
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

.pay-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: #fff;
  cursor: pointer;
}

.pay-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.bottom-links {
  flex-shrink: 0;
  padding: 16px 0 14px;
  display: flex;
  justify-content: center;
}

.link-text {
  font-size: 0.85rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  transition: color 0.15s;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.link-text:hover {
  color: var(--qs-primary-dark);
}

/* ── 二维码弹窗 ── */
.qr-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.qr-modal {
  background: #fff;
  border-radius: 16px;
  width: 340px;
  max-width: 90vw;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.qr-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid #f1f5f9;
}

.qr-header h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: #1e293b;
}

.qr-close {
  width: 28px;
  height: 28px;
  border: none;
  background: #f1f5f9;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  transition: all 0.15s;
}

.qr-close:hover {
  background: #e2e8f0;
  color: #1e293b;
}

.qr-body {
  padding: 20px;
}

.qr-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.qr-img {
  width: 220px;
  height: 220px;
  border-radius: 8px;
}

.qr-hint {
  margin: 12px 0 4px;
  font-size: 0.85rem;
  color: #64748b;
}

.qr-amount {
  font-size: 1.3rem;
  font-weight: 700;
  color: var(--qs-primary);
  margin: 4px 0;
}

.qr-polling {
  font-size: 0.8rem;
  color: #94a3b8;
  margin-top: 8px;
}

.qr-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 0;
  gap: 8px;
}

.qr-result i {
  font-size: 2.5rem;
}

.qr-result p {
  font-size: 1rem;
  font-weight: 600;
  margin: 0;
}

.qr-result--ok i,
.qr-result--ok p {
  color: var(--qs-primary);
}

.qr-result--fail i,
.qr-result--fail p {
  color: #dc2626;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.25s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
