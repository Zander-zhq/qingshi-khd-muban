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
  <div class="hud-dashboard">
    <!-- Profile HUD Card -->
    <section class="profile-hud">
      <div class="ph-grid-bg"></div>
      <div class="ph-content">
        <div class="ph-left">
          <div class="ph-avatar">
            <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="ph-avatar-img" alt="" />
            <span v-else class="ph-avatar-text">{{ displayName.charAt(0) }}</span>
            <div class="ph-avatar-ring"></div>
          </div>
          <div class="ph-info">
            <div class="ph-welcome">{{ brand.brand_name }} · {{ brand.product_name }} {{ VERSION }}</div>
            <h1 class="ph-name">{{ displayName }}</h1>
            <div class="ph-status">
              <span class="status-dot"></span>
              <span>在线</span>
            </div>
          </div>
        </div>
        <div class="ph-right">
          <div class="ph-clock">{{ currentTime }}</div>
          <div class="ph-date">{{ currentDate }}</div>
        </div>
      </div>
    </section>

    <section v-if="userStore.userInfo?.invite_code && hasInviteReward" class="invite-card">
      <div class="invite-grid-bg"></div>
      <div class="invite-content">
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
      </div>
    </section>

    <!-- 1×4 Horizontal Metric Strip -->
    <section class="metric-strip">
      <div class="strip-card">
        <div class="sc-icon sc-icon--cyan"><i class="pi pi-calendar"></i></div>
        <div class="sc-body">
          <span class="sc-label">{{ memberInfo.label }}</span>
          <strong class="sc-value">{{ memberInfo.value }}</strong>
          <div v-if="memberInfo.warning" class="sc-warning">⚠ {{ memberInfo.warning }}</div>
        </div>
        <div class="sc-glow sc-glow--cyan"></div>
      </div>

      <div class="strip-card">
        <div class="sc-icon sc-icon--purple"><i class="pi pi-user"></i></div>
        <div class="sc-body">
          <span class="sc-label">当前账号</span>
          <strong class="sc-value">{{ userStore.userInfo?.username || '未设置' }}</strong>
        </div>
        <div class="sc-glow sc-glow--purple"></div>
      </div>

      <div class="strip-card">
        <div class="sc-icon sc-icon--green"><i class="pi pi-phone"></i></div>
        <div class="sc-body">
          <span class="sc-label">手机号</span>
          <strong class="sc-value">{{ userStore.userInfo?.phone || '未绑定' }}</strong>
        </div>
        <div class="sc-glow sc-glow--green"></div>
      </div>

      <div class="strip-card">
        <div class="sc-icon sc-icon--amber"><i class="pi pi-envelope"></i></div>
        <div class="sc-body">
          <span class="sc-label">邮箱</span>
          <strong class="sc-value">{{ userStore.userInfo?.email || '未绑定' }}</strong>
        </div>
        <div class="sc-glow sc-glow--amber"></div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.hud-dashboard {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ═══ Profile HUD Card ═══ */
.profile-hud {
  position: relative;
  border-radius: 16px;
  padding: 32px 30px;
  background: linear-gradient(135deg, #0F172A 0%, #1E293B 50%, #0F172A 100%);
  border: 1px solid #334155;
  overflow: hidden;
  box-shadow: 0 0 30px rgba(34, 211, 238, 0.06);
}

.ph-grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(34, 211, 238, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(34, 211, 238, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
}

.ph-content {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.ph-left {
  display: flex;
  align-items: center;
  gap: 20px;
}

.ph-avatar {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  position: relative;
  flex-shrink: 0;
}

.ph-avatar-img {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
}

.ph-avatar-text {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: linear-gradient(135deg, #22D3EE, #0891B2);
  color: #0F172A;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.8rem;
  font-weight: 700;
}

.ph-avatar-ring {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  border: 2px solid rgba(34, 211, 238, 0.4);
  pointer-events: none;
  box-shadow: 0 0 16px rgba(34, 211, 238, 0.2);
}

.ph-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.ph-welcome {
  font-size: 0.75rem;
  color: #22D3EE;
  letter-spacing: 0.08em;
}

.ph-name {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 700;
  color: #E2E8F0;
}

.ph-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: #64748B;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #22D3EE;
  box-shadow: 0 0 6px rgba(34, 211, 238, 0.6);
}

.ph-right { text-align: right; }

.ph-clock {
  font-size: 2.4rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: #22D3EE;
  text-shadow: 0 0 20px rgba(34, 211, 238, 0.4);
  font-variant-numeric: tabular-nums;
}

.ph-date {
  font-size: 0.82rem;
  color: #64748B;
  margin-top: 2px;
}

/* ═══ 1×4 Metric Strip ═══ */
.metric-strip {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.strip-card {
  position: relative;
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 12px;
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  overflow: hidden;
  transition: all 0.25s;
}

.strip-card:hover {
  border-color: rgba(34, 211, 238, 0.3);
  transform: translateY(-2px);
}

.sc-icon {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.sc-icon i { font-size: 1rem; }

.sc-icon--cyan { background: rgba(34, 211, 238, 0.1); border: 1px solid rgba(34, 211, 238, 0.2); color: #22D3EE; }
.sc-icon--purple { background: rgba(168, 85, 247, 0.1); border: 1px solid rgba(168, 85, 247, 0.2); color: #A855F7; }
.sc-icon--green { background: rgba(34, 197, 94, 0.1); border: 1px solid rgba(34, 197, 94, 0.2); color: #22C55E; }
.sc-icon--amber { background: rgba(245, 158, 11, 0.1); border: 1px solid rgba(245, 158, 11, 0.2); color: #F59E0B; }

.sc-body { flex: 1; min-width: 0; }
.sc-label { display: block; font-size: 0.72rem; color: #64748B; margin-bottom: 3px; }
.sc-value { display: block; font-size: 0.92rem; color: #E2E8F0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.sc-glow {
  position: absolute;
  top: 0;
  right: 0;
  width: 60px;
  height: 60px;
  border-radius: 50%;
  filter: blur(30px);
  opacity: 0.06;
  pointer-events: none;
}

.sc-glow--cyan { background: #22D3EE; }
.sc-glow--purple { background: #A855F7; }
.sc-glow--green { background: #22C55E; }
.sc-glow--amber { background: #F59E0B; }

.sc-warning { margin-top: 4px; font-size: 0.68rem; color: #fbbf24; font-weight: 500; }

/* ═══ Invite Card ═══ */
.invite-card {
  position: relative;
  border-radius: 12px;
  padding: 22px 24px;
  background: #1E293B;
  border: 1px solid #334155;
  overflow: hidden;
}

.invite-grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(34, 211, 238, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(34, 211, 238, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
}

.invite-content {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
}

.invite-title {
  font-size: 1rem;
  font-weight: 600;
  color: #22D3EE;
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.invite-title i { font-size: 1.1rem; }

.invite-desc {
  font-size: 0.82rem;
  color: #94A3B8;
  line-height: 1.6;
}

.invite-desc strong {
  color: #22D3EE;
  font-weight: 600;
}

.invite-right {
  flex-shrink: 0;
  text-align: center;
}

.invite-code-label {
  font-size: 0.72rem;
  color: #64748B;
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
  color: #22D3EE;
  letter-spacing: 0.12em;
  background: rgba(34, 211, 238, 0.08);
  padding: 6px 14px;
  border-radius: 8px;
  border: 1px dashed rgba(34, 211, 238, 0.3);
  font-family: 'Consolas', 'Monaco', monospace;
  text-shadow: 0 0 10px rgba(34, 211, 238, 0.3);
}

.invite-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 14px;
  border: 1px solid rgba(34, 211, 238, 0.4);
  border-radius: 8px;
  background: rgba(34, 211, 238, 0.1);
  color: #22D3EE;
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.invite-copy-btn:hover {
  background: rgba(34, 211, 238, 0.2);
  border-color: rgba(34, 211, 238, 0.6);
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.15);
}
</style>
