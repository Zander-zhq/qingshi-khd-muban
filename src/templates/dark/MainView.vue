<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useUserStore } from '../../stores/user'
import { getBrand, VERSION } from '../../brand'

const brand = getBrand()
const APP_BRAND = brand.brand_name
const APP_PRODUCT = brand.product_name
const APP_VERSION = VERSION

const userStore = useUserStore()

const displayName = computed(() => userStore.userInfo?.username || '用户')

const memberInfo = computed(() => {
  const info = userStore.userInfo
  if (!info) return { label: '会员状态', value: '未知' }

  if (info.app_mode === 'points') {
    return { label: '剩余积分', value: info.fen !== undefined ? String(info.fen) : '0' }
  }

  if (info.vip_expire_at) {
    const d = new Date(info.vip_expire_at)
    const now = new Date()
    const isExpired = d.getTime() < now.getTime()
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    return { label: '到期时间', value: isExpired ? `已到期 (${dateStr})` : dateStr }
  }

  return { label: '到期时间', value: '永久' }
})

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
      <div class="hero-grid-bg"></div>
      <div class="hero-content">
        <div>
          <div class="hero-kicker">{{ APP_BRAND }} · {{ APP_PRODUCT }} {{ APP_VERSION }}</div>
          <h1>欢迎回来，{{ displayName }}</h1>
        </div>
        <div class="hero-time">
          <div class="time-value">{{ currentTime }}</div>
          <div class="time-date">{{ currentDate }}</div>
        </div>
      </div>
    </section>

    <section class="metric-grid">
      <div class="metric-card">
        <div class="metric-icon">
          <i class="pi pi-calendar"></i>
        </div>
        <div class="metric-body">
          <span>{{ memberInfo.label }}</span>
          <strong>{{ memberInfo.value }}</strong>
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-icon">
          <i class="pi pi-user"></i>
        </div>
        <div class="metric-body">
          <span>当前账号</span>
          <strong>{{ userStore.userInfo?.username || '未设置' }}</strong>
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-icon">
          <i class="pi pi-phone"></i>
        </div>
        <div class="metric-body">
          <span>手机号</span>
          <strong>{{ userStore.userInfo?.phone || '未绑定' }}</strong>
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-icon">
          <i class="pi pi-envelope"></i>
        </div>
        <div class="metric-body">
          <span>邮箱</span>
          <strong>{{ userStore.userInfo?.email || '未绑定' }}</strong>
        </div>
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
  position: relative;
  min-height: 140px;
  border-radius: 16px;
  padding: 28px 30px;
  background: linear-gradient(135deg, #0F172A 0%, #1E293B 50%, #0F172A 100%);
  border: 1px solid #334155;
  color: #E2E8F0;
  overflow: hidden;
  box-shadow: 0 0 30px rgba(34, 211, 238, 0.08);
}

.hero-grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(34, 211, 238, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(34, 211, 238, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
}

.hero-content {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
}

.hero-kicker {
  font-size: 0.82rem;
  letter-spacing: 0.12em;
  color: #22D3EE;
  margin-bottom: 10px;
}

.hero-card h1 {
  font-size: 1.8rem;
  margin: 0;
  color: #E2E8F0;
}

.hero-time {
  min-width: 180px;
  text-align: right;
}

.time-value {
  font-size: 2.2rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: #22D3EE;
  text-shadow: 0 0 20px rgba(34, 211, 238, 0.4);
}

.time-date {
  font-size: 0.82rem;
  color: #64748B;
  margin-top: 4px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.metric-card {
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 14px;
  padding: 18px 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  transition: all 0.2s;
}

.metric-card:hover {
  border-color: rgba(34, 211, 238, 0.3);
  box-shadow: 0 0 16px rgba(34, 211, 238, 0.08);
}

.metric-icon {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  background: rgba(34, 211, 238, 0.08);
  border: 1px solid rgba(34, 211, 238, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.metric-icon i {
  font-size: 1.1rem;
  color: #22D3EE;
}

.metric-body {
  flex: 1;
  min-width: 0;
}

.metric-body span {
  display: block;
  color: #64748B;
  font-size: 0.78rem;
  margin-bottom: 4px;
}

.metric-body strong {
  color: #E2E8F0;
  font-size: 1.1rem;
  display: block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
