<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useUserStore } from '../stores/user'

const APP_BRAND = '青拾'
const APP_PRODUCT = '视频下载'
const APP_VERSION = 'V1.1.1'

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
      <div>
        <div class="hero-kicker">{{ APP_BRAND }} · {{ APP_PRODUCT }} {{ APP_VERSION }}</div>
        <h1>欢迎回来，{{ displayName }}</h1>
      </div>
      <div class="hero-time">
        <div class="time-value">{{ currentTime }}</div>
        <div class="time-date">{{ currentDate }}</div>
      </div>
    </section>

    <section class="metric-grid">
      <div class="metric-card">
        <span>{{ memberInfo.label }}</span>
        <strong>{{ memberInfo.value }}</strong>
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

</style>
