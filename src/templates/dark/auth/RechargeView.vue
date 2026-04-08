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

      <div class="body">
        <div class="form-card">
          <h2 class="card-title">充值中心</h2>
          <p class="card-subtitle">{{ hasOnlinePay ? '在线充值 / 卡密充值' : '输入卡密为账号充值' }}</p>

          <!-- 账号输入（共用） -->
          <div class="field-box" style="margin-bottom:10px">
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
          <div v-if="activeTab === 'card' || !hasOnlinePay" class="form-area">
            <form class="form" @submit.prevent="handleRecharge">
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
  background: #0F172A;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #0F172A;
  overflow-y: auto;
}

.body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
  min-height: 0;
}

.form-card {
  width: 100%;
  max-width: 380px;
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 12px;
  padding: 28px 32px 20px;
}

.card-title {
  margin: 0 0 4px;
  font-size: 1.3rem;
  font-weight: 700;
  color: #E2E8F0;
  text-align: center;
  letter-spacing: 0.08em;
}

.card-subtitle {
  margin: 0 0 20px;
  font-size: 0.82rem;
  color: #64748B;
  text-align: center;
}

.form-area {
  display: flex;
  flex-direction: column;
  align-items: center;
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

.field-box :deep(.field-input) {
  width: 100%;
  height: 44px;
  font-size: 0.92rem;
  border: 1px solid #334155;
  border-radius: 10px;
  background: #0F172A;
  padding: 0 16px;
  color: #E2E8F0;
  transition: all 0.2s;
}

.field-box :deep(.field-input:focus) {
  border-color: #22D3EE;
  background: #0F172A;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3);
  outline: none;
}

.field-box :deep(.field-input::placeholder) {
  color: #475569;
}

/* ── Tab 切换 ── */
.rc-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 14px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid #334155;
}

.rc-tab {
  flex: 1;
  padding: 10px 0;
  font-size: 0.85rem;
  font-weight: 500;
  border: none;
  background: #0F172A;
  color: #64748B;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.2s;
}

.rc-tab.active {
  background: rgba(34, 211, 238, 0.15);
  color: #22D3EE;
  font-weight: 600;
}

.rc-tab:not(.active):hover {
  background: rgba(34, 211, 238, 0.05);
}

/* ── 在线支付区域 ── */
.rc-online {
  width: 100%;
}

.rc-loading,
.rc-empty {
  text-align: center;
  padding: 20px 0;
  color: #64748B;
  font-size: 0.85rem;
}

.rc-packages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
  gap: 8px;
  margin-bottom: 14px;
}

.rc-pkg-card {
  border: 1px solid #334155;
  border-radius: 10px;
  padding: 12px 8px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: #0F172A;
}

.rc-pkg-card:hover {
  border-color: rgba(34, 211, 238, 0.4);
  background: rgba(34, 211, 238, 0.05);
}

.rc-pkg-card.selected {
  border-color: #22D3EE;
  background: rgba(34, 211, 238, 0.1);
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.2);
}

.rc-pkg-name {
  font-size: 0.88rem;
  font-weight: 600;
  color: #E2E8F0;
  margin-bottom: 3px;
}

.rc-pkg-desc {
  font-size: 0.72rem;
  color: #64748B;
  margin-bottom: 5px;
}

.rc-pkg-price {
  color: #22D3EE;
  font-weight: 700;
}

.rc-price-symbol {
  font-size: 0.72rem;
}

.rc-price-value {
  font-size: 1.05rem;
}

/* ── 支付方式 ── */
.rc-pay-methods {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.rc-pay-label {
  font-size: 0.78rem;
  color: #64748B;
  white-space: nowrap;
}

.rc-pay-options {
  display: flex;
  gap: 8px;
}

.rc-pay-opt {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  border: 1px solid #334155;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.78rem;
  color: #E2E8F0;
  transition: all 0.2s;
  background: #0F172A;
}

.rc-pay-opt input[type="radio"] { display: none; }

.rc-pay-opt.active {
  border-color: #22D3EE;
  background: rgba(34, 211, 238, 0.1);
}

.rc-pay-opt:hover {
  border-color: rgba(34, 211, 238, 0.4);
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
  color: #f87171;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
}

.msg-ok {
  color: #22D3EE;
  background: rgba(34, 211, 238, 0.1);
  border: 1px solid rgba(34, 211, 238, 0.25);
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
  border-radius: 10px;
  margin-top: 4px;
  background: transparent !important;
  border: 1px solid #22D3EE !important;
  color: #22D3EE !important;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3);
  transition: all 0.25s;
}

.submit-btn:hover {
  background: rgba(34, 211, 238, 0.15) !important;
  box-shadow: 0 0 20px rgba(34, 211, 238, 0.5);
  transform: translateY(-1px);
}

.pay-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  cursor: pointer;
}

.pay-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  transform: none;
}

.bottom-links {
  flex-shrink: 0;
  padding: 16px 0 0;
  display: flex;
  justify-content: center;
  border-top: 1px solid #334155;
  margin-top: 16px;
}

.link-text {
  font-size: 0.85rem;
  color: #22D3EE;
  text-decoration: none;
  font-weight: 500;
  transition: all 0.15s;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.link-text:hover {
  color: #67E8F9;
  text-shadow: 0 0 8px rgba(34, 211, 238, 0.4);
}

/* ── 二维码弹窗 ── */
.qr-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.qr-modal {
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 14px;
  width: 340px;
  max-width: 90vw;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.qr-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid #334155;
}

.qr-header h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: #E2E8F0;
}

.qr-close {
  width: 28px;
  height: 28px;
  border: none;
  background: #0F172A;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748B;
  transition: all 0.15s;
}

.qr-close:hover {
  background: #334155;
  color: #E2E8F0;
}

.qr-body { padding: 20px; }

.qr-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.qr-img {
  width: 200px;
  height: 200px;
  border-radius: 8px;
}

.qr-hint {
  margin: 12px 0 4px;
  font-size: 0.85rem;
  color: #64748B;
}

.qr-amount {
  font-size: 1.3rem;
  font-weight: 700;
  color: #22D3EE;
  margin: 4px 0;
}

.qr-polling {
  font-size: 0.8rem;
  color: #475569;
  margin-top: 8px;
}

.qr-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 0;
  gap: 8px;
}

.qr-result i { font-size: 2.5rem; }
.qr-result p { font-size: 1rem; font-weight: 600; margin: 0; }

.qr-result--ok i,
.qr-result--ok p { color: #22D3EE; }

.qr-result--fail i,
.qr-result--fail p { color: #f87171; }

.modal-enter-active,
.modal-leave-active { transition: opacity 0.25s ease; }
.modal-enter-from,
.modal-leave-to { opacity: 0; }
</style>
