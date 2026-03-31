<script setup lang="ts">
import { computed } from 'vue'
import { useUserStore } from '../stores/user'

const userStore = useUserStore()

const displayName = computed(() => {
  if (userStore.userInfo?.username) return userStore.userInfo.username
  return '用户'
})
</script>

<template>
  <div class="dashboard-page">
    <section class="hero-card">
      <div>
        <div class="hero-kicker">青拾客户端</div>
        <h1>欢迎回来，{{ displayName }}</h1>
        <p>主窗口已经独立出来了，后续功能页面都会加载在右侧工作区。</p>
      </div>
      <div class="hero-time">
        <div class="time-label">当前状态</div>
        <div class="time-value">在线</div>
      </div>
    </section>

    <section class="metric-grid">
      <div class="metric-card">
        <span>登录状态</span>
        <strong>{{ userStore.isLoggedIn ? '有效' : '无效' }}</strong>
      </div>
      <div class="metric-card">
        <span>当前账号</span>
        <strong>{{ userStore.userInfo?.username || '未返回资料' }}</strong>
      </div>
      <div class="metric-card">
        <span>手机号</span>
        <strong>{{ userStore.userInfo?.phone || '未返回' }}</strong>
      </div>
      <div class="metric-card">
        <span>邮箱</span>
        <strong>{{ userStore.userInfo?.email || '未返回' }}</strong>
      </div>
    </section>

    <section class="panel-grid">
      <div class="panel-card">
        <div class="panel-title">功能区</div>
        <p>这里后面可以接视频任务、下载中心、配置管理等页面。</p>
      </div>
      <div class="panel-card">
        <div class="panel-title">开发说明</div>
        <p>当前已经完成独立主窗口、左侧菜单、顶部用户区和内容工作区的基础骨架。</p>
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
  min-height: 160px;
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
  letter-spacing: 0.16em;
  opacity: 0.88;
  margin-bottom: 10px;
}

.hero-card h1 {
  font-size: 2rem;
  margin-bottom: 10px;
}

.hero-card p {
  font-size: 0.92rem;
  line-height: 1.7;
  max-width: 520px;
  color: rgba(255, 255, 255, 0.92);
}

.hero-time {
  min-width: 160px;
  text-align: right;
}

.time-label {
  font-size: 0.8rem;
  opacity: 0.82;
  margin-bottom: 10px;
}

.time-value {
  font-size: 2rem;
  font-weight: 700;
}

.metric-grid,
.panel-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.metric-card,
.panel-card {
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

.panel-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 8px;
}

.panel-card p {
  color: #64748b;
  line-height: 1.7;
  font-size: 0.88rem;
}
</style>
