<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useRegister } from '../../../composables/useRegister'

const {
  router, phone, password, confirmPassword, inviteCode,
  loading, errMsg, successMsg, clearMsg, handleRegister,
} = useRegister()
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="scroll-body">
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="banner-title">注册账号</div>
      </div>

      <div class="body">
        <form class="form" @submit.prevent="handleRegister">
          <div class="field-box">
            <InputText
              v-model="phone"
              placeholder="手机号"
              class="field-input"
              maxlength="11"
              @input="clearMsg"
            />
          </div>

          <div class="field-box">
            <Password
              v-model="password"
              placeholder="密码（6-18位）"
              toggleMask
              class="field-pw"
              inputClass="field-input"
              autocomplete="new-password"
              @input="clearMsg"
            />
          </div>

          <div class="field-box">
            <Password
              v-model="confirmPassword"
              placeholder="确认密码"
              :feedback="false"
              toggleMask
              class="field-pw"
              inputClass="field-input"
              autocomplete="new-password"
              @input="clearMsg"
            />
          </div>

          <div class="field-box">
            <InputText
              v-model="inviteCode"
              placeholder="邀请码（选填）"
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

          <Button type="submit" label="注 册" :loading="loading" class="submit-btn" />
        </form>

        <div class="bottom-links">
          <a href="#" class="link-text" @click.prevent="router.push('/login')">
            <i class="pi pi-arrow-left" style="font-size: 0.7rem"></i>
            返回登录
          </a>
          <span class="bottom-sep">|</span>
          <a href="#" class="link-text" @click.prevent="router.push('/recharge')">
            卡密充值
          </a>
        </div>
      </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.window-shell {
  height: 100vh;
  width: 100vw;
  background: #fff;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
  overflow: hidden;
}

.window-content :deep(.app-titlebar) { flex-shrink: 0; }

.scroll-body {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.window-content :deep(.app-titlebar.titlebar-compact) {
  position: relative;
}

.banner {
  height: 100px;
  position: relative;
  background: var(--qs-bg-gradient);
  flex-shrink: 0;
  overflow: visible;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bc {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.07);
  pointer-events: none;
}
.bc-1 { width: 140px; height: 140px; top: -50px; right: -20px; }
.bc-2 { width: 80px; height: 80px; bottom: -30px; left: 10px; }

.banner-title {
  position: relative;
  z-index: 1;
  font-size: 1.4rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.12em;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.1);
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 20px 36px 10px;
  min-height: 0;
}

.form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field-box {
  width: 100%;
}

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%;
  height: 38px;
  font-size: 0.88rem;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  background: #f8fafb;
  padding: 0 14px;
  transition: all 0.2s;
}

:deep(.field-pw) {
  width: 100%;
}

.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: var(--qs-primary-light);
  background: #fff;
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.08);
}

.msg-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.78rem;
  padding: 6px 10px;
  border-radius: 8px;
}

.msg-err {
  color: #dc2626;
  background: #fef2f2;
  border: 1px solid #fecaca;
}

.msg-ok {
  color: var(--qs-primary-dark);
  background: #f0fdfa;
  border: 1px solid #99f6e4;
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
  height: 40px;
  font-size: 0.95rem;
  font-weight: 600;
  border-radius: 20px;
  margin-top: 2px;
  background: var(--qs-bg-gradient) !important;
  border: none !important;
  box-shadow: 0 4px 16px rgba(13, 148, 136, 0.3);
  transition: all 0.2s;
}

.submit-btn:hover {
  box-shadow: 0 6px 24px rgba(13, 148, 136, 0.4);
  transform: translateY(-1px);
}

.bottom-links {
  margin-top: auto;
  padding: 6px 0 8px;
  display: flex;
  justify-content: center;
}

.link-text {
  font-size: 0.85rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  transition: color 0.15s;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.link-text:hover {
  color: var(--qs-primary-dark);
}

.bottom-sep {
  color: #cbd5e1;
  font-size: 0.8rem;
}

.bottom-links {
  gap: 8px;
}
</style>
