<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from './TitleBar.vue'
import { useAuth } from '../../composables/useAuth'
import { useProfile } from '../../composables/useProfile'
import { useRecharge } from '../../composables/useRecharge'
import { useChangePassword } from '../../composables/useChangePassword'
import { useUnbindDevice } from '../../composables/useUnbindDevice'
import { getUnbindTip } from '../../utils/config'
import { startHeartbeat } from '../../utils/heartbeat'

const route = useRoute()

const {
  userStore, displayName,
  showBanned, bannedTitle, bannedMsg, bannedCountdown,
  showExpired, expiredMsg, forceExpired,
  initHeartbeat, destroyHeartbeat, handleLogout,
} = useAuth()

const {
  showProfileModal, profileNickname, profileAcctno, profileEmail, profileEmailCode,
  emailEditing, emailSending, emailCooldown, emailBtnText,
  profileAvatarPreview, profileLoading, profileErrMsg, profileSuccessMsg,
  avatarInputRef,
  openProfileModal, closeProfileModal, triggerAvatarSelect, onAvatarSelected,
  handleSendEmailCode, submitProfile,
} = useProfile()

const {
  showRechargeModal, rechargeCardKey, rechargeLoading, rechargeErrMsg, rechargeSuccessMsg,
  openRechargeModal, closeRechargeModal, submitRecharge,
} = useRecharge(forceExpired, showExpired)

const {
  showChangePwdModal, cpOldPwd, cpNewPwd, cpConfirmPwd, cpLoading, cpErrMsg, cpSuccessMsg,
  openChangePwdModal, submitChangePassword,
} = useChangePassword()

const {
  showUnbindModal, unbindLoading, unbindErrMsg, unbindSuccessMsg,
  openUnbindModal, submitUnbind,
} = useUnbindDevice()

const menuItems = [
  { label: '仪表盘', icon: 'pi pi-home', path: '/main/dashboard' },
]

const pageTitle = computed(() => {
  if (route.path === '/main/dashboard') return '仪表盘'
  return '主页'
})

const showUserMenu = ref(false)

function toggleUserMenu(e: MouseEvent) {
  e.stopPropagation()
  showUserMenu.value = !showUserMenu.value
}

function closeUserMenu() {
  showUserMenu.value = false
}

function handleNavigate(path: string) {
  import('vue-router').then(({ useRouter }) => useRouter().push(path))
}

function handleEditProfile() {
  showUserMenu.value = false
  openProfileModal()
}

function handleRecharge() {
  showUserMenu.value = false
  openRechargeModal()
}

function handleChangePassword() {
  showUserMenu.value = false
  openChangePwdModal()
}

function handleUnbindDevice() {
  showUserMenu.value = false
  openUnbindModal()
}

async function onLogout() {
  showUserMenu.value = false
  await handleLogout()
}

onMounted(() => {
  initHeartbeat()
  document.addEventListener('click', closeUserMenu)

  const info = userStore.userInfo
  if (info && (!info.acctno || !info.email)) {
    setTimeout(() => handleEditProfile(), 500)
  }
})

onUnmounted(() => {
  destroyHeartbeat()
  document.removeEventListener('click', closeUserMenu)
})
</script>

<template>
  <div class="layout-root">
    <TitleBar variant="full" :title="pageTitle" />

    <!-- 顶部导航栏 -->
    <nav class="topnav">
      <div class="topnav-left">
        <div class="topnav-logo">
          <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="logo-img" alt="" />
          <span v-else class="logo-placeholder">{{ displayName.charAt(0) }}</span>
        </div>
        <div class="topnav-brand">
          <strong>{{ displayName }}</strong>
          <span>{{ userStore.isLoggedIn ? '已登录' : '未登录' }}</span>
        </div>
      </div>

      <div class="topnav-center">
        <button
          v-for="item in menuItems"
          :key="item.path"
          type="button"
          class="topnav-item"
          :class="{ active: route.path === item.path }"
          @click="handleNavigate(item.path)"
        >
          <i :class="item.icon"></i>
          <span>{{ item.label }}</span>
        </button>
      </div>

      <div class="topnav-right">
        <div class="user-card-wrap">
          <button type="button" class="user-card" @click="toggleUserMenu($event)">
            <div class="user-avatar">
              <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="avatar-img" alt="" />
              <span v-else>{{ displayName.charAt(0) }}</span>
            </div>
            <i class="pi pi-chevron-down user-card-arrow" :class="{ 'arrow--open': showUserMenu }"></i>
          </button>

          <Transition name="dropdown">
            <div v-if="showUserMenu" class="user-dropdown" @click.stop>
              <div class="ud-header">
                <div class="ud-avatar">
                  <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" class="avatar-img" alt="" />
                  <span v-else>{{ displayName.charAt(0) }}</span>
                </div>
                <div class="ud-info">
                  <strong>{{ displayName }}</strong>
                  <span>{{ userStore.userInfo?.phone || userStore.userInfo?.email || '未绑定手机/邮箱' }}</span>
                </div>
              </div>
              <div class="ud-divider"></div>
              <button class="ud-item" @click="handleEditProfile">
                <i class="pi pi-user-edit"></i> 编辑资料
              </button>
              <button class="ud-item" @click="handleRecharge">
                <i class="pi pi-credit-card"></i> 卡密充值
              </button>
              <button class="ud-item" @click="handleChangePassword">
                <i class="pi pi-lock"></i> 修改密码
              </button>
              <button class="ud-item" @click="handleUnbindDevice">
                <i class="pi pi-link"></i> 解绑设备
              </button>
              <div class="ud-divider"></div>
              <button class="ud-item ud-item--danger" @click="onLogout">
                <i class="pi pi-sign-out"></i> 退出登录
              </button>
            </div>
          </Transition>
        </div>
      </div>
    </nav>

    <!-- 内容区 -->
    <main class="content-area">
      <RouterView />
    </main>

    <!-- 卡密充值弹窗 -->
    <Transition name="modal">
      <div v-if="showRechargeModal" class="modal-overlay" @click.self="closeRechargeModal">
        <div class="modal-box">
          <div class="modal-header">
            <h3>卡密充值</h3>
            <button type="button" class="modal-close" @click="closeRechargeModal"><i class="pi pi-times"></i></button>
          </div>
          <form class="modal-body" @submit.prevent="submitRecharge">
            <div class="modal-field">
              <label>当前账号</label>
              <div class="modal-acctno">{{ userStore.userInfo?.phone || userStore.userInfo?.username || '-' }}</div>
            </div>
            <div class="modal-field">
              <label>卡密</label>
              <InputText v-model="rechargeCardKey" placeholder="请输入卡密" class="modal-input" />
            </div>
            <Transition name="fade">
              <div v-if="rechargeErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ rechargeErrMsg }}</div>
              <div v-else-if="rechargeSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ rechargeSuccessMsg }}</div>
            </Transition>
            <Button type="submit" label="充 值" :loading="rechargeLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 编辑资料弹窗 -->
    <Transition name="modal">
      <div v-if="showProfileModal" class="modal-overlay" @click.self="closeProfileModal">
        <div class="modal-box modal-box--profile">
          <div class="modal-header">
            <h3>编辑资料</h3>
            <button type="button" class="modal-close" @click="closeProfileModal"><i class="pi pi-times"></i></button>
          </div>
          <form class="modal-body" @submit.prevent="submitProfile">
            <div class="profile-avatar-area">
              <div class="profile-avatar-wrap" @click="triggerAvatarSelect">
                <img v-if="profileAvatarPreview" :src="profileAvatarPreview" class="profile-avatar-img" alt="头像" />
                <div v-else class="profile-avatar-placeholder">{{ displayName.charAt(0) }}</div>
                <div class="profile-avatar-overlay"><i class="pi pi-camera"></i></div>
              </div>
              <input ref="avatarInputRef" type="file" accept="image/jpeg,image/png,image/gif,image/webp" style="display:none" @change="onAvatarSelected" />
              <span class="profile-avatar-hint">点击更换头像</span>
            </div>

            <div class="modal-field">
              <label>昵称</label>
              <InputText v-model="profileNickname" placeholder="请输入昵称" class="modal-input" maxlength="128" />
            </div>
            <div class="modal-field">
              <label>账号</label>
              <InputText v-model="profileAcctno" placeholder="可用来登录的账号（2-32位）" class="modal-input" maxlength="32" />
            </div>
            <div class="modal-field">
              <label>邮箱 <span class="field-hint">可用于找回密码</span></label>
              <template v-if="!emailEditing && userStore.userInfo?.email">
                <div class="email-bound">
                  <span>{{ userStore.userInfo.email }}</span>
                  <button type="button" class="email-change-btn" @click="emailEditing = true; profileEmail = ''">更换</button>
                </div>
              </template>
              <template v-else>
                <InputText v-model="profileEmail" placeholder="请输入邮箱地址" class="modal-input" type="email" />
                <div class="email-code-row">
                  <InputText v-model="profileEmailCode" placeholder="验证码" class="modal-input email-code-input" maxlength="6" />
                  <Button type="button" :label="emailBtnText" :disabled="emailCooldown > 0" :loading="emailSending" class="email-code-btn" @click="handleSendEmailCode" />
                </div>
              </template>
            </div>
            <Transition name="fade">
              <div v-if="profileErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ profileErrMsg }}</div>
              <div v-else-if="profileSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ profileSuccessMsg }}</div>
            </Transition>
            <Button type="submit" label="保存" :loading="profileLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 修改密码弹窗 -->
    <Transition name="modal">
      <div v-if="showChangePwdModal" class="modal-overlay" @click.self="showChangePwdModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>修改密码</h3>
            <button type="button" class="modal-close" @click="showChangePwdModal = false"><i class="pi pi-times"></i></button>
          </div>
          <form class="modal-body" @submit.prevent="submitChangePassword">
            <div class="modal-field">
              <label>原密码</label>
              <InputText v-model="cpOldPwd" placeholder="请输入原密码" class="modal-input" type="password" />
            </div>
            <div class="modal-field">
              <label>新密码</label>
              <InputText v-model="cpNewPwd" placeholder="请输入新密码（至少6位）" class="modal-input" type="password" />
            </div>
            <div class="modal-field">
              <label>确认新密码</label>
              <InputText v-model="cpConfirmPwd" placeholder="请再次输入新密码" class="modal-input" type="password" />
            </div>
            <Transition name="fade">
              <div v-if="cpErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ cpErrMsg }}</div>
              <div v-else-if="cpSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ cpSuccessMsg }}</div>
            </Transition>
            <Button type="submit" label="确认修改" :loading="cpLoading" class="modal-submit" />
          </form>
        </div>
      </div>
    </Transition>

    <!-- 解绑设备弹窗 -->
    <Transition name="modal">
      <div v-if="showUnbindModal" class="modal-overlay" @click.self="showUnbindModal = false">
        <div class="modal-box">
          <div class="modal-header">
            <h3>解绑当前设备</h3>
            <button type="button" class="modal-close" @click="showUnbindModal = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="modal-body" style="text-align:center">
            <p style="font-size:0.9rem;color:#9a3412;margin:0 0 20px;line-height:1.6">{{ getUnbindTip() }}</p>
            <Transition name="fade">
              <div v-if="unbindErrMsg" class="modal-msg modal-msg--err"><i class="pi pi-exclamation-circle"></i>{{ unbindErrMsg }}</div>
              <div v-else-if="unbindSuccessMsg" class="modal-msg modal-msg--ok"><i class="pi pi-check-circle"></i>{{ unbindSuccessMsg }}</div>
            </Transition>
            <Button label="确认解绑" :loading="unbindLoading" class="modal-submit" style="margin-top:8px" @click="submitUnbind" />
          </div>
        </div>
      </div>
    </Transition>

    <!-- 到期续费弹窗 -->
    <Transition name="modal">
      <div v-if="showExpired" class="modal-overlay">
        <div class="modal-box">
          <div class="modal-header"><h3>使用到期</h3></div>
          <div class="modal-body" style="text-align:center">
            <div class="expired-icon"><i class="pi pi-clock"></i></div>
            <p class="expired-msg">{{ expiredMsg }}</p>
            <div style="display:flex;gap:10px">
              <Button label="卡密充值" class="modal-submit" style="flex:1" @click="showExpired = false; handleRecharge()" />
              <Button label="退出登录" class="modal-submit" severity="secondary" style="flex:1;background:#64748b !important" @click="forceExpired = false; showExpired = false; onLogout()" />
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 账号禁用遮罩 -->
    <div v-if="showBanned" class="banned-overlay">
      <div class="banned-box">
        <div class="banned-icon"><i class="pi pi-ban"></i></div>
        <h2 class="banned-title">{{ bannedTitle }}</h2>
        <p class="banned-msg">{{ bannedMsg }}</p>
        <p class="banned-countdown">{{ bannedCountdown }} 秒后自动退出登录</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.layout-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #FFF7ED;
  animation: page-fade-in 0.35s ease-out;
}

/* ── 顶部导航栏 ── */
.topnav {
  display: flex;
  align-items: center;
  height: 60px;
  padding: 0 24px;
  background: #fff;
  border-bottom: 1px solid #fed7aa;
  flex-shrink: 0;
  gap: 20px;
}

.topnav-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.topnav-logo {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: linear-gradient(135deg, #fb923c, #F97316);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.95rem;
  flex-shrink: 0;
  overflow: hidden;
}

.logo-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.logo-placeholder {
  font-size: 1rem;
  font-weight: 700;
}

.topnav-brand {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.topnav-brand strong {
  font-size: 0.92rem;
  color: #7C2D12;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.topnav-brand span {
  font-size: 0.72rem;
  color: #c2410c;
  opacity: 0.6;
}

.topnav-center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.topnav-item {
  height: 40px;
  padding: 0 20px;
  border: none;
  border-radius: 12px;
  background: transparent;
  color: #9a3412;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 0.88rem;
  font-weight: 500;
  transition: all 0.15s ease;
}

.topnav-item i {
  font-size: 1rem;
}

.topnav-item:hover {
  background: #FFF7ED;
  color: #F97316;
}

.topnav-item.active {
  background: linear-gradient(135deg, rgba(249, 115, 22, 0.12), rgba(234, 88, 12, 0.18));
  color: #EA580C;
  font-weight: 600;
  box-shadow: inset 0 0 0 1px rgba(249, 115, 22, 0.15);
}

.topnav-right {
  display: flex;
  align-items: center;
}

/* ── User Card ── */
.user-card-wrap { position: relative; }

.user-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 12px;
  border: none;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s;
}

.user-card:hover { background: #FFF7ED; }

.user-avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: linear-gradient(135deg, #fb923c, #F97316);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.85rem;
  flex-shrink: 0;
  overflow: hidden;
}

.avatar-img { width: 100%; height: 100%; object-fit: cover; }

.user-card-arrow { font-size: 0.7rem; color: #c2410c; transition: transform 0.2s; }
.arrow--open { transform: rotate(180deg); }

/* ── User Dropdown ── */
.user-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  min-width: 220px;
  background: #fff;
  border: 1px solid #fed7aa;
  border-radius: 16px;
  box-shadow: 0 10px 36px rgba(249, 115, 22, 0.12);
  padding: 6px 0;
  z-index: 500;
}

.ud-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px 10px;
}

.ud-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, #fb923c, #F97316);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1rem;
  flex-shrink: 0;
  overflow: hidden;
}

.ud-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.ud-info strong { font-size: 0.92rem; color: #7C2D12; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ud-info span { font-size: 0.75rem; color: #c2410c; opacity: 0.6; }
.ud-divider { height: 1px; background: #FFF7ED; margin: 4px 0; }

.ud-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 16px;
  border: none;
  background: none;
  color: #7C2D12;
  font-size: 0.86rem;
  cursor: pointer;
  transition: background 0.12s;
  white-space: nowrap;
}

.ud-item:hover { background: #FFF7ED; }
.ud-item i { font-size: 0.92rem; width: 18px; text-align: center; color: #c2410c; }
.ud-item--danger { color: #dc2626; }
.ud-item--danger i { color: #dc2626; }
.ud-item--danger:hover { background: #fef2f2; }

.dropdown-enter-active, .dropdown-leave-active { transition: all 0.2s ease; }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(-6px); }

/* ── Content Area ── */
.content-area {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 20px 24px;
}

/* ── Modals ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(124, 45, 18, 0.3);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-box {
  width: 400px;
  max-width: 90vw;
  background: #fff;
  border-radius: 20px;
  box-shadow: 0 20px 60px rgba(249, 115, 22, 0.15);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #FFF7ED;
}

.modal-header h3 { margin: 0; font-size: 1.1rem; font-weight: 700; color: #7C2D12; }

.modal-close {
  width: 32px; height: 32px; border: none; background: transparent;
  color: #c2410c; border-radius: 10px; cursor: pointer;
  display: flex; align-items: center; justify-content: center; transition: all 0.15s;
}

.modal-close:hover { background: #FFF7ED; color: #7C2D12; }

.modal-body { padding: 20px 24px 24px; display: flex; flex-direction: column; gap: 16px; }

.modal-field { display: flex; flex-direction: column; gap: 6px; }
.modal-field label { font-size: 0.82rem; font-weight: 600; color: #9a3412; }

.modal-acctno {
  height: 42px; line-height: 42px; font-size: 0.9rem; color: #7C2D12;
  font-weight: 600; padding: 0 14px; background: #FFF7ED;
  border-radius: 16px; border: 1.5px solid #fed7aa;
}

.modal-field :deep(.modal-input) {
  width: 100%; height: 42px; font-size: 0.9rem;
  border: 1.5px solid #fed7aa; border-radius: 16px; background: #fffbf5;
  padding: 0 14px; transition: all 0.2s; color: #7C2D12;
}

.modal-field :deep(.modal-input:focus) {
  border-color: #F97316; background: #fff;
  box-shadow: 0 0 0 3px rgba(249, 115, 22, 0.1);
}

.modal-msg { display: flex; align-items: center; gap: 6px; font-size: 0.82rem; padding: 8px 12px; border-radius: 12px; }
.modal-msg--err { color: #dc2626; background: #fef2f2; border: 1px solid #fecaca; }
.modal-msg--ok { color: #EA580C; background: #fff7ed; border: 1px solid #fed7aa; }

.modal-submit {
  width: 100%; height: 42px; font-size: 0.95rem; font-weight: 600;
  border-radius: 21px;
  background: linear-gradient(135deg, #F97316, #EA580C) !important;
  border: none !important;
  box-shadow: 0 4px 16px rgba(249, 115, 22, 0.3);
  transition: all 0.2s; margin-top: 4px;
}

.modal-submit:hover { box-shadow: 0 6px 24px rgba(249, 115, 22, 0.4); transform: translateY(-1px); }

.modal-enter-active, .modal-leave-active { transition: opacity 0.2s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-active .modal-box, .modal-leave-active .modal-box { transition: transform 0.2s ease; }
.modal-enter-from .modal-box { transform: scale(0.95); }
.modal-leave-to .modal-box { transform: scale(0.95); }

.fade-enter-active, .fade-leave-active { transition: all 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }

/* ── Profile Modal ── */
.modal-box--profile { width: 440px; }

.profile-avatar-area { display: flex; flex-direction: column; align-items: center; gap: 8px; padding-bottom: 8px; }

.profile-avatar-wrap {
  position: relative; width: 80px; height: 80px; border-radius: 50%;
  cursor: pointer; overflow: hidden;
  box-shadow: 0 2px 12px rgba(249, 115, 22, 0.15);
}

.profile-avatar-img { width: 100%; height: 100%; object-fit: cover; }

.profile-avatar-placeholder {
  width: 100%; height: 100%;
  background: linear-gradient(135deg, #fb923c, #F97316);
  color: #fff; display: flex; align-items: center; justify-content: center;
  font-size: 2rem; font-weight: 700;
}

.profile-avatar-overlay {
  position: absolute; inset: 0; background: rgba(0, 0, 0, 0.35);
  display: flex; align-items: center; justify-content: center;
  opacity: 0; transition: opacity 0.2s;
}

.profile-avatar-overlay i { color: #fff; font-size: 1.3rem; }
.profile-avatar-wrap:hover .profile-avatar-overlay { opacity: 1; }
.profile-avatar-hint { font-size: 0.75rem; color: #c2410c; opacity: 0.5; }

/* ── Banned Overlay ── */
.banned-overlay {
  position: fixed; inset: 0; background: rgba(124, 45, 18, 0.85);
  backdrop-filter: blur(6px); display: flex; align-items: center;
  justify-content: center; z-index: 9999;
}

.banned-box {
  text-align: center; padding: 48px 56px; background: #fff;
  border-radius: 24px; box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3); max-width: 420px;
}

.banned-icon {
  width: 72px; height: 72px; border-radius: 50%; background: #fef2f2;
  display: flex; align-items: center; justify-content: center; margin: 0 auto 20px;
}

.banned-icon i { font-size: 2rem; color: #dc2626; }
.banned-title { margin: 0 0 12px; font-size: 1.3rem; font-weight: 700; color: #dc2626; }
.banned-msg { margin: 0 0 20px; font-size: 0.9rem; color: #9a3412; line-height: 1.6; }
.banned-countdown { margin: 0; font-size: 0.85rem; color: #c2410c; opacity: 0.6; }

.expired-icon {
  width: 64px; height: 64px; border-radius: 50%; background: #fff7ed;
  display: flex; align-items: center; justify-content: center; margin: 0 auto 16px;
}

.expired-icon i { font-size: 1.8rem; color: #F97316; }
.expired-msg { font-size: 0.92rem; color: #9a3412; line-height: 1.6; margin: 0 0 20px; }

.field-hint { font-weight: 400; font-size: 0.72rem; color: #c2410c; opacity: 0.5; margin-left: 4px; }

.email-bound {
  display: flex; align-items: center; justify-content: space-between;
  height: 42px; padding: 0 14px; background: #FFF7ED;
  border-radius: 16px; border: 1.5px solid #fed7aa; font-size: 0.9rem; color: #7C2D12;
}

.email-change-btn {
  border: none; background: none; color: #F97316; font-size: 0.82rem;
  font-weight: 600; cursor: pointer; padding: 4px 8px; border-radius: 8px;
  transition: background 0.15s;
}

.email-change-btn:hover { background: rgba(249, 115, 22, 0.08); }

.email-code-row { display: flex; gap: 10px; align-items: center; margin-top: 8px; }
.email-code-row :deep(.email-code-input) { flex: 1; min-width: 0; }

.email-code-btn {
  flex-shrink: 0; height: 42px; font-size: 0.82rem; font-weight: 600;
  border-radius: 16px; white-space: nowrap; padding: 0 14px;
  background: linear-gradient(135deg, #F97316, #EA580C) !important;
  border: none !important; color: #fff !important; transition: all 0.2s;
}

.email-code-btn:disabled { opacity: 0.6; }
</style>
