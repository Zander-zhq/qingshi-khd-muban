<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { redeemCardApi } from '../../../api/auth'
import { getAppCredentials } from '../../../utils/config'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'
import { useGuestPay } from '../../../composables/useGuestPay'

const brand = getBrand()
const brandLogo = getBrandLogo()
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

function clearMsg() { errMsg.value = ''; successMsg.value = '' }

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
    const res = await redeemCardApi({ app_id: appId.value, acctno: acctno.value.trim(), card_key: cardKey.value.trim() })
    const cardType = (res as any).card_type || ''; const expireAt = (res as any).vip_expire_at || ''
    let msg = '充值成功！'
    if (cardType) msg += ` (${cardType})`
    if (expireAt) { const d = new Date(expireAt); msg += `  到期：${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}` }
    successMsg.value = msg; cardKey.value = ''
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) { errMsg.value = err instanceof Error ? err.message : '充值失败' }
  finally { loading.value = false }
}

function handleStartPay() {
  if (!acctno.value.trim()) { payErrMsg.value = '请输入账号（手机号）'; return }
  startPay(acctno.value)
}
</script>

<template>
  <div class="page-shell">
    <TitleBar variant="auth" />
    <div class="page-split">
      <div class="left-panel">
        <div class="lp-deco lp-deco-1"></div>
        <div class="lp-deco lp-deco-2"></div>
        <div class="lp-content">
          <img :src="brandLogo" alt="" class="lp-logo" />
          <h1 class="lp-title">{{ brand.brand_name }}</h1>
          <p class="lp-sub">{{ brand.product_name }}</p>
          <div class="lp-divider"></div>
          <p class="lp-hint">{{ hasOnlinePay ? '在线充值 / 卡密充值' : '输入卡密即可充值会员' }}</p>
        </div>
        <div class="lp-ver">{{ VERSION }}</div>
      </div>

      <div class="right-panel">
        <div class="rp-content">
          <h2 class="rp-title">充值中心</h2>
          <p class="rp-desc">请输入账号进行充值</p>

          <!-- 账号输入（共用） -->
          <div class="field" style="margin-bottom:10px">
            <div class="input-box">
              <i class="pi pi-user input-icon"></i>
              <InputText v-model="acctno" placeholder="账号（手机号）" class="gk-input" @input="clearMsg" />
            </div>
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
                <div v-if="payErrMsg" class="msg msg--err">
                  <i class="pi pi-exclamation-circle"></i>{{ payErrMsg }}
                </div>
                <div v-else-if="paySuccessMsg" class="msg msg--ok">
                  <i class="pi pi-check-circle"></i>{{ paySuccessMsg }}
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
            <div class="field">
              <div class="input-box">
                <i class="pi pi-credit-card input-icon"></i>
                <InputText v-model="cardKey" placeholder="卡密" class="gk-input" @input="clearMsg" />
              </div>
            </div>
            <Transition name="fade">
              <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
              <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
            </Transition>
            <Button type="submit" label="充 值" :loading="loading" class="submit-btn" />
          </form>

          <div class="rp-links">
            <a href="#" @click.prevent="router.push('/login')"><i class="pi pi-arrow-left"></i> 返回登录</a>
          </div>
        </div>
      </div>
    </div>

    <!-- 支付二维码弹窗 -->
    <Transition name="modal">
      <div v-if="showQrModal" class="qr-overlay" @click.self="closeQrModal">
        <div class="qr-modal">
          <div class="qr-header">
            <h3>{{ selectedPayMethod === 'wechat' ? '微信支付' : '支付宝支付' }}</h3>
            <button type="button" class="qr-close" @click="closeQrModal"><i class="pi pi-times"></i></button>
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
              <a href="#" class="rp-links" @click.prevent="closeQrModal(); router.push('/login')">返回登录</a>
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
</template>

<style scoped>
@import './auth-shared.css';

/* ── Tab 切换 ── */
.rc-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 14px;
  border-radius: 4px;
  overflow: hidden;
  border: 1px solid #DCDFE6;
}

.rc-tab {
  flex: 1;
  padding: 8px 0;
  font-size: 0.82rem;
  font-weight: 500;
  border: none;
  background: #fff;
  color: #909399;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  transition: all 0.2s;
}

.rc-tab.active {
  background: #F97316;
  color: #fff;
  font-weight: 600;
}

.rc-tab:not(.active):hover {
  background: #FFF7ED;
}

/* ── 在线支付区域 ── */
.rc-loading,
.rc-empty {
  text-align: center;
  padding: 20px 0;
  color: #909399;
  font-size: 0.82rem;
}

.rc-packages {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
  gap: 8px;
  margin-bottom: 12px;
}

.rc-pkg-card {
  border: 1px solid #DCDFE6;
  border-radius: 4px;
  padding: 10px 8px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: #fff;
}

.rc-pkg-card:hover {
  border-color: #FDBA74;
  background: #FFF7ED;
}

.rc-pkg-card.selected {
  border-color: #F97316;
  background: #FFF7ED;
  box-shadow: 0 0 0 2px rgba(249, 115, 22, 0.15);
}

.rc-pkg-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: #303133;
  margin-bottom: 3px;
}

.rc-pkg-desc {
  font-size: 0.7rem;
  color: #909399;
  margin-bottom: 5px;
}

.rc-pkg-price {
  color: #F97316;
  font-weight: 700;
}

.rc-price-symbol {
  font-size: 0.7rem;
}

.rc-price-value {
  font-size: 1rem;
}

/* ── 支付方式 ── */
.rc-pay-methods {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.rc-pay-label {
  font-size: 0.75rem;
  color: #909399;
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
  padding: 5px 12px;
  border: 1px solid #DCDFE6;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.75rem;
  transition: all 0.2s;
}

.rc-pay-opt input[type="radio"] { display: none; }

.rc-pay-opt.active {
  border-color: #F97316;
  background: #FFF7ED;
}

.rc-pay-opt:hover {
  border-color: #FDBA74;
}

.pay-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  color: #fff !important;
  cursor: pointer;
}

.pay-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* ── 二维码弹窗 ── */
.qr-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.qr-modal {
  background: #fff;
  border-radius: 8px;
  width: 340px;
  max-width: 90vw;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.18);
  overflow: hidden;
}

.qr-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #EBEEF5;
}

.qr-header h3 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: #303133;
}

.qr-close {
  width: 26px;
  height: 26px;
  border: none;
  background: #f5f7fa;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
  transition: all 0.15s;
}

.qr-close:hover {
  background: #e4e7ed;
  color: #303133;
}

.qr-body { padding: 18px; }

.qr-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.qr-img {
  width: 200px;
  height: 200px;
  border-radius: 6px;
}

.qr-hint {
  margin: 10px 0 4px;
  font-size: 0.82rem;
  color: #909399;
}

.qr-amount {
  font-size: 1.2rem;
  font-weight: 700;
  color: #F97316;
  margin: 4px 0;
}

.qr-polling {
  font-size: 0.75rem;
  color: #C0C4CC;
  margin-top: 6px;
}

.qr-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 18px 0;
  gap: 8px;
}

.qr-result i { font-size: 2.2rem; }
.qr-result p { font-size: 0.95rem; font-weight: 600; margin: 0; }

.qr-result--ok i,
.qr-result--ok p { color: #67C23A; }

.qr-result--fail i,
.qr-result--fail p { color: #F56C6C; }

.modal-enter-active,
.modal-leave-active { transition: opacity 0.2s ease; }
.modal-enter-from,
.modal-leave-to { opacity: 0; }
</style>
