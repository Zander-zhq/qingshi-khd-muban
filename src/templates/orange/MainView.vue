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
  if (!info) return { label: '会员状态', value: '未知', icon: 'pi-star' }

  if (info.app_mode === 'points') {
    return { label: '剩余积分', value: info.fen !== undefined ? String(info.fen) : '0', icon: 'pi-bolt' }
  }

  if (info.vip_expire_at) {
    const d = new Date(info.vip_expire_at)
    const now = new Date()
    const isExpired = d.getTime() < now.getTime()
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    return { label: '到期时间', value: isExpired ? `已到期 (${dateStr})` : dateStr, icon: 'pi-calendar' }
  }

  return { label: '到期时间', value: '永久', icon: 'pi-calendar' }
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
    <!-- 欢迎横幅 -->
    <section class="hero-card">
      <div class="hero-left">
        <div class="hero-kicker">{{ APP_BRAND }} · {{ APP_PRODUCT }} {{ APP_VERSION }}</div>
        <h1 class="hero-title">欢迎回来，{{ displayName }}</h1>
        <p class="hero-desc">祝您使用愉快，开始今天的任务吧</p>
      </div>
      <div class="hero-right">
        <div class="time-value">{{ currentTime }}</div>
        <div class="time-date">{{ currentDate }}</div>
      </div>
    </section>

    <!-- 信息卡片网格 -->
    <section class="card-grid">
      <div class="info-card info-card--accent">
        <div class="card-icon-wrap card-icon--orange">
          <i class="pi" :class="memberInfo.icon"></i>
        </div>
        <div class="card-body">
          <span class="card-label">{{ memberInfo.label }}</span>
          <strong class="card-value">{{ memberInfo.value }}</strong>
        </div>
      </div>

      <div class="info-card">
        <div class="card-icon-wrap card-icon--amber">
          <i class="pi pi-user"></i>
        </div>
        <div class="card-body">
          <span class="card-label">当前账号</span>
          <strong class="card-value">{{ userStore.userInfo?.username || '未设置' }}</strong>
        </div>
      </div>

      <div class="info-card">
        <div class="card-icon-wrap card-icon--warm">
          <i class="pi pi-phone"></i>
        </div>
        <div class="card-body">
          <span class="card-label">手机号</span>
          <strong class="card-value">{{ userStore.userInfo?.phone || '未绑定' }}</strong>
        </div>
      </div>

      <div class="info-card">
        <div class="card-icon-wrap card-icon--brown">
          <i class="pi pi-envelope"></i>
        </div>
        <div class="card-body">
          <span class="card-label">邮箱</span>
          <strong class="card-value">{{ userStore.userInfo?.email || '未绑定' }}</strong>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dashboard-page {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* ── Hero Card ── */
.hero-card {
  min-height: 160px;
  border-radius: 24px;
  padding: 32px 36px;
  background: linear-gradient(135deg, #F97316 0%, #EA580C 50%, #C2410C 100%);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  box-shadow: 0 18px 40px rgba(249, 115, 22, 0.25);
  position: relative;
  overflow: hidden;
}

.hero-card::before {
  content: '';
  position: absolute;
  width: 200px;
  height: 200px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.05);
  top: -60px;
  right: -40px;
  pointer-events: none;
}

.hero-card::after {
  content: '';
  position: absolute;
  width: 120px;
  height: 120px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.04);
  bottom: -30px;
  left: 40px;
  pointer-events: none;
}

.hero-left {
  position: relative;
  z-index: 1;
}

.hero-kicker {
  font-size: 0.82rem;
  letter-spacing: 0.12em;
  opacity: 0.8;
  margin-bottom: 10px;
}

.hero-title {
  font-size: 1.8rem;
  margin: 0 0 6px;
  font-weight: 800;
}

.hero-desc {
  margin: 0;
  font-size: 0.88rem;
  opacity: 0.75;
}

.hero-right {
  min-width: 180px;
  text-align: right;
  position: relative;
  z-index: 1;
}

.time-value {
  font-size: 2.4rem;
  font-weight: 700;
  letter-spacing: 0.05em;
}

.time-date {
  font-size: 0.82rem;
  opacity: 0.8;
  margin-top: 4px;
}

/* ── Card Grid ── */
.card-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}

.info-card {
  background: #fff;
  border-radius: 20px;
  padding: 22px 24px;
  box-shadow: 0 6px 24px rgba(249, 115, 22, 0.06);
  display: flex;
  align-items: center;
  gap: 16px;
  transition: all 0.2s;
  border: 1px solid rgba(254, 215, 170, 0.4);
}

.info-card:hover {
  box-shadow: 0 12px 36px rgba(249, 115, 22, 0.1);
  transform: translateY(-2px);
}

.card-icon-wrap {
  width: 48px;
  height: 48px;
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.card-icon-wrap i {
  font-size: 1.2rem;
}

.card-icon--orange {
  background: linear-gradient(135deg, #FFF7ED, #FFEDD5);
  color: #F97316;
}

.card-icon--amber {
  background: linear-gradient(135deg, #FFFBEB, #FEF3C7);
  color: #D97706;
}

.card-icon--warm {
  background: linear-gradient(135deg, #FFF1F2, #FFE4E6);
  color: #E11D48;
}

.card-icon--brown {
  background: linear-gradient(135deg, #F5F3FF, #EDE9FE);
  color: #7C3AED;
}

.card-body {
  flex: 1;
  min-width: 0;
}

.card-label {
  display: block;
  color: #9a3412;
  font-size: 0.82rem;
  margin-bottom: 6px;
  opacity: 0.7;
}

.card-value {
  color: #7C2D12;
  font-size: 1.15rem;
  font-weight: 700;
  display: block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
