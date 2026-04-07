<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useUserStore } from '../../stores/user'
import { getBrand, VERSION, formatReward } from '../../brand'

const brand = getBrand()
const userStore = useUserStore()
const inviterReward = formatReward(brand.invite_inviter_reward_type, brand.invite_inviter_reward_value)
const inviteeReward = formatReward(brand.invite_invitee_reward_type, brand.invite_invitee_reward_value)
const hasInviteReward = !!(inviterReward || inviteeReward)

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

const displayName = computed(() => userStore.userInfo?.username || '用户')

const memberInfo = computed(() => {
  const info = userStore.userInfo
  if (!info) return { label: '会员状态', value: '未知', icon: 'pi-star', warning: '' }
  if (info.app_mode === 'points') {
    const fen = info.fen ?? 0
    return { label: '剩余积分', value: String(fen), icon: 'pi-bolt', warning: fen < 30 ? '积分不足，请尽快充值' : '' }
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
    return { label: '到期时间', value: isExpired ? `已到期 (${dateStr})` : dateStr, icon: 'pi-calendar', warning }
  }
  return { label: '到期时间', value: '永久', icon: 'pi-calendar', warning: '' }
})

const currentTime = ref('')
const currentDate = ref('')
let clockTimer: ReturnType<typeof setInterval> | null = null

function updateClock() {
  const now = new Date()
  currentTime.value = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`
  const weekdays = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']
  currentDate.value = `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日${weekdays[now.getDay()]}`
}

onMounted(() => { updateClock(); clockTimer = setInterval(updateClock, 1000) })
onUnmounted(() => { if (clockTimer) clearInterval(clockTimer) })
</script>

<template>
  <div class="dashboard">
    <section v-if="userStore.userInfo?.invite_code && hasInviteReward" class="invite-card">
      <div class="invite-left">
        <div class="invite-title"><i class="pi pi-users"></i> 邀请好友赚奖励</div>
        <div class="invite-desc">
          <span v-if="inviterReward">每邀请一位新用户，您可获得 <strong>{{ inviterReward }}</strong></span>
          <span v-if="inviterReward && inviteeReward">；</span>
          <span v-if="inviteeReward">好友注册即得 <strong>{{ inviteeReward }}</strong></span>
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

    <!-- Stats-first: 1×4 Row -->
    <section class="stats-row">
      <div class="stat-card stat-card--primary">
        <div class="stat-stripe"></div>
        <div class="stat-icon"><i class="pi" :class="memberInfo.icon"></i></div>
        <div class="stat-body">
          <span class="stat-label">{{ memberInfo.label }}</span>
          <strong class="stat-value">{{ memberInfo.value }}</strong>
          <div v-if="memberInfo.warning" class="stat-warning">⚠ {{ memberInfo.warning }}</div>
        </div>
      </div>

      <div class="stat-card stat-card--warning">
        <div class="stat-stripe"></div>
        <div class="stat-icon"><i class="pi pi-user"></i></div>
        <div class="stat-body">
          <span class="stat-label">当前账号</span>
          <strong class="stat-value">{{ userStore.userInfo?.username || '未设置' }}</strong>
        </div>
      </div>

      <div class="stat-card stat-card--success">
        <div class="stat-stripe"></div>
        <div class="stat-icon"><i class="pi pi-phone"></i></div>
        <div class="stat-body">
          <span class="stat-label">手机号</span>
          <strong class="stat-value">{{ userStore.userInfo?.phone || '未绑定' }}</strong>
        </div>
      </div>

      <div class="stat-card stat-card--info">
        <div class="stat-stripe"></div>
        <div class="stat-icon"><i class="pi pi-envelope"></i></div>
        <div class="stat-body">
          <span class="stat-label">邮箱</span>
          <strong class="stat-value">{{ userStore.userInfo?.email || '未绑定' }}</strong>
        </div>
      </div>
    </section>

    <!-- Welcome card below -->
    <section class="welcome-card">
      <div class="wc-left">
        <div class="wc-badge">{{ brand.brand_name }} · {{ brand.product_name }} {{ VERSION }}</div>
        <h1 class="wc-title">欢迎回来，{{ displayName }}</h1>
        <p class="wc-desc">祝您使用愉快，开始今天的任务吧</p>
      </div>
      <div class="wc-right">
        <div class="wc-time">{{ currentTime }}</div>
        <div class="wc-date">{{ currentDate }}</div>
      </div>
      <div class="wc-deco"></div>
    </section>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ═══ 1×4 Stats Row (stats-first layout) ═══ */
.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
}

.stat-card {
  background: #fff;
  border-radius: 4px;
  padding: 18px 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  position: relative;
  overflow: hidden;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  transition: box-shadow 0.2s, transform 0.2s;
}

.stat-card:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
  transform: translateY(-1px);
}

.stat-stripe {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
}

.stat-card--primary .stat-stripe { background: #F97316; }
.stat-card--warning .stat-stripe { background: #E6A23C; }
.stat-card--success .stat-stripe { background: #67C23A; }
.stat-card--info .stat-stripe { background: #409EFF; }

.stat-icon {
  width: 40px;
  height: 40px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.stat-icon i { font-size: 1rem; }

.stat-card--primary .stat-icon { background: #FFF7ED; color: #F97316; }
.stat-card--warning .stat-icon { background: #FDF6EC; color: #E6A23C; }
.stat-card--success .stat-icon { background: #F0F9EB; color: #67C23A; }
.stat-card--info .stat-icon { background: #ECF5FF; color: #409EFF; }

.stat-body { flex: 1; min-width: 0; }

.stat-label {
  display: block;
  font-size: 0.72rem;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  display: block;
  font-size: 0.95rem;
  font-weight: 600;
  color: #303133;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ═══ Welcome Card (below stats) ═══ */
.welcome-card {
  border-radius: 4px;
  padding: 28px 32px;
  background: linear-gradient(135deg, #F97316 0%, #EA580C 60%, #C2410C 100%);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  position: relative;
  overflow: hidden;
}

.wc-deco {
  position: absolute;
  width: 180px;
  height: 180px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.06);
  top: -50px;
  right: -30px;
  pointer-events: none;
}

.wc-left { position: relative; z-index: 1; }

.wc-badge {
  display: inline-block;
  font-size: 0.72rem;
  background: rgba(255, 255, 255, 0.2);
  padding: 2px 10px;
  border-radius: 3px;
  margin-bottom: 10px;
  letter-spacing: 0.05em;
}

.wc-title {
  font-size: 1.4rem;
  margin: 0 0 6px;
  font-weight: 700;
}

.wc-desc {
  margin: 0;
  font-size: 0.82rem;
  opacity: 0.8;
}

.wc-right {
  min-width: 160px;
  text-align: right;
  position: relative;
  z-index: 1;
}

.wc-time {
  font-size: 2rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  font-variant-numeric: tabular-nums;
}

.wc-date {
  font-size: 0.78rem;
  opacity: 0.8;
  margin-top: 4px;
}

.stat-warning { margin-top: 4px; font-size: 0.7rem; color: #d97706; font-weight: 500; }

/* ═══ Invite Card ═══ */
.invite-card {
  background: #fff;
  border-radius: 4px;
  padding: 20px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  border-left: 3px solid #F97316;
}

.invite-title {
  font-size: 1rem;
  font-weight: 600;
  color: #EA580C;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.invite-title i { font-size: 1.1rem; }

.invite-desc {
  font-size: 0.82rem;
  color: #78716c;
  line-height: 1.6;
}

.invite-desc strong {
  color: #F97316;
  font-weight: 600;
}

.invite-right {
  flex-shrink: 0;
  text-align: center;
}

.invite-code-label {
  font-size: 0.72rem;
  color: #909399;
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
  color: #EA580C;
  letter-spacing: 0.12em;
  background: #FFF7ED;
  padding: 6px 14px;
  border-radius: 4px;
  border: 1px dashed #FDBA74;
  font-family: 'Consolas', 'Monaco', monospace;
}

.invite-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 14px;
  border: none;
  border-radius: 4px;
  background: #F97316;
  color: #fff;
  font-size: 0.82rem;
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
}

.invite-copy-btn:hover { background: #EA580C; }
</style>
