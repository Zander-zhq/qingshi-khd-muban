<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useUserStore } from '../../stores/user'
import { getBrand, VERSION, formatReward } from '../../brand'

const brand = getBrand()
const inviterReward = formatReward(brand.invite_inviter_reward_type, brand.invite_inviter_reward_value)
const inviteeReward = formatReward(brand.invite_invitee_reward_type, brand.invite_invitee_reward_value)
const hasInviteReward = !!(inviterReward || inviteeReward)
const APP_BRAND = brand.brand_name
const APP_PRODUCT = brand.product_name
const APP_VERSION = VERSION

const userStore = useUserStore()

const displayName = computed(() => userStore.userInfo?.username || '用户')

const memberInfo = computed(() => {
  const info = userStore.userInfo
  if (!info) return { label: '会员状态', value: '未知', warning: '' }

  if (info.app_mode === 'points') {
    const fen = info.fen ?? 0
    return { label: '剩余积分', value: String(fen), warning: fen < 30 ? '积分不足，请尽快充值' : '' }
  }

  if (info.vip_expire_at) {
    const d = new Date(info.vip_expire_at)
    const now = new Date()
    const isExpired = d.getTime() < now.getTime()
    const diffDays = Math.ceil((d.getTime() - now.getTime()) / (1000 * 60 * 60 * 24))
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`
    let warning = ''
    if (isExpired) warning = '已到期，请尽快充值'
    else if (diffDays <= 3) warning = `还有${diffDays}天将会到期，请尽快充值`
    return { label: '到期时间', value: isExpired ? `已到期 (${dateStr})` : dateStr, warning }
  }

  return { label: '到期时间', value: '永久', warning: '' }
})

const copySuccess = ref(false)
async function copyInviteCode() {
  const code = userStore.userInfo?.invite_code
  if (!code) return
  try {
    await navigator.clipboard.writeText(code)
    copySuccess.value = true
    setTimeout(() => copySuccess.value = false, 2000)
  } catch { /* fallback */ }
}

const currentTime = ref('')
const currentDate = ref('')
let clockTimer: ReturnType<typeof setInterval> | null = null

function updateClock() {
  const now = new Date()
  const h = String(now.getHours()).padStart(2, '0')
  const m = String(now.getMinutes()).padStart(2, '0')
  const s = String(now.getSeconds()).padStart(2, '0')
  currentTime.value = `${h}:${m}:${s}`

  const year = now.getFullYear()
  const month = now.getMonth() + 1
  const day = now.getDate()
  const weekdays = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']
  currentDate.value = `${year}年${month}月${day}日${weekdays[now.getDay()]}`
}

onMounted(() => {
  updateClock()
  clockTimer = setInterval(updateClock, 1000)
})

onUnmounted(() => {
  if (clockTimer) clearInterval(clockTimer)
})
</script>

<template>
  <div class="dashboard-page">
    <section class="hero-card">
      <div>
        <div class="hero-kicker">{{ APP_BRAND }} · {{ APP_PRODUCT }} {{ APP_VERSION }}</div>
        <h1>欢迎回来，{{ displayName }}</h1>
      </div>
      <div class="hero-time">
        <div class="time-value">{{ currentTime }}</div>
        <div class="time-date">{{ currentDate }}</div>
      </div>
    </section>

    <section v-if="userStore.userInfo?.invite_code && hasInviteReward" class="invite-card">
      <div class="invite-left">
        <div class="invite-title"><i class="pi pi-users"></i> 邀请好友</div>
        <div class="invite-desc">
          <span v-if="inviterReward">邀请新用户注册，您可获得 <strong>{{ inviterReward }}</strong></span>
          <span v-if="inviterReward && inviteeReward">，</span>
          <span v-if="inviteeReward">好友也能获得 <strong>{{ inviteeReward }}</strong></span>
        </div>
      </div>
      <div class="invite-right">
        <div class="invite-code-label">我的邀请码</div>
        <div class="invite-code-row">
          <span class="invite-code">{{ userStore.userInfo.invite_code }}</span>
          <button class="invite-copy-btn" @click="copyInviteCode">
            <i class="pi" :class="copySuccess ? 'pi-check' : 'pi-copy'"></i>
            {{ copySuccess ? '已复制' : '复制' }}
          </button>
        </div>
      </div>
    </section>

    <section class="metric-grid">
      <div class="metric-card" :class="{ 'metric-card--warn': memberInfo.warning }">
        <span>{{ memberInfo.label }}</span>
        <strong>{{ memberInfo.value }}</strong>
        <div v-if="memberInfo.warning" class="metric-warning">⚠ {{ memberInfo.warning }}</div>
      </div>
      <div class="metric-card">
        <span>当前账号</span>
        <strong>{{ userStore.userInfo?.username || '未设置' }}</strong>
      </div>
      <div class="metric-card">
        <span>手机号</span>
        <strong>{{ userStore.userInfo?.phone || '未绑定' }}</strong>
      </div>
      <div class="metric-card">
        <span>邮箱</span>
        <strong>{{ userStore.userInfo?.email || '未绑定' }}</strong>
      </div>
    </section>

  </div>
</template>

<style scoped>
.dashboard-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.hero-card {
  min-height: 140px;
  border-radius: 20px;
  padding: 28px 30px;
  background: linear-gradient(135deg, #0f766e 0%, #0d9488 42%, #2dd4bf 100%);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  box-shadow: 0 18px 40px rgba(13, 148, 136, 0.22);
}

.hero-kicker {
  font-size: 0.82rem;
  letter-spacing: 0.12em;
  opacity: 0.88;
  margin-bottom: 10px;
}

.hero-card h1 {
  font-size: 1.8rem;
  margin: 0;
}

.hero-time {
  min-width: 180px;
  text-align: right;
}

.time-value {
  font-size: 2.2rem;
  font-weight: 700;
  letter-spacing: 0.05em;
}

.time-date {
  font-size: 0.82rem;
  opacity: 0.82;
  margin-top: 4px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.metric-card {
  background: #fff;
  border-radius: 18px;
  padding: 18px 20px;
  box-shadow: 0 10px 30px rgba(15, 23, 42, 0.06);
}

.metric-card span {
  display: block;
  color: #64748b;
  font-size: 0.82rem;
  margin-bottom: 8px;
}

.metric-card strong {
  color: #0f172a;
  font-size: 1.2rem;
}

.metric-card--warn { border-color: #fbbf24; }
.metric-warning { margin-top: 6px; font-size: 0.72rem; color: #d97706; font-weight: 500; }

/* ═══ Invite Card ═══ */
.invite-card {
  background: linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%);
  border: 1px solid #a7f3d0;
  border-radius: 18px;
  padding: 20px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
}

.invite-title {
  font-size: 1rem;
  font-weight: 600;
  color: #065f46;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.invite-title i { font-size: 1.1rem; }

.invite-desc {
  font-size: 0.82rem;
  color: #047857;
  line-height: 1.6;
}

.invite-desc strong {
  color: #059669;
  font-weight: 600;
}

.invite-right {
  flex-shrink: 0;
  text-align: center;
}

.invite-code-label {
  font-size: 0.72rem;
  color: #6b7280;
  margin-bottom: 6px;
}

.invite-code-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.invite-code {
  font-size: 1.2rem;
  font-weight: 700;
  color: #065f46;
  letter-spacing: 0.12em;
  background: #fff;
  padding: 6px 14px;
  border-radius: 8px;
  border: 1px dashed #a7f3d0;
  font-family: 'Consolas', 'Monaco', monospace;
}

.invite-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 14px;
  border: none;
  border-radius: 8px;
  background: #059669;
  color: #fff;
  font-size: 0.82rem;
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
}

.invite-copy-btn:hover { background: #047857; }

</style>
