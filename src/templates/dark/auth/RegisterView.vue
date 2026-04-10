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
      <div class="body">
        <div class="form-card">
          <h2 class="card-title">注册账号</h2>
          <p class="card-subtitle">创建您的新账号</p>

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
  </div>
</template>

<style scoped>
.window-shell {
  height: 100vh;
  width: 100vw;
  background: #0F172A;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #0F172A;
  overflow: hidden;
}

.window-content :deep(.app-titlebar) {
  flex-shrink: 0;
}

.scroll-body {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
  min-height: 0;
}

.form-card {
  width: 100%;
  max-width: 380px;
  background: #1E293B;
  border: 1px solid #334155;
  border-radius: 12px;
  padding: 28px 32px 20px;
}

.card-title {
  margin: 0 0 4px;
  font-size: 1.3rem;
  font-weight: 700;
  color: #E2E8F0;
  text-align: center;
  letter-spacing: 0.08em;
}

.card-subtitle {
  margin: 0 0 20px;
  font-size: 0.82rem;
  color: #64748B;
  text-align: center;
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
  height: 42px;
  font-size: 0.9rem;
  border: 1px solid #334155;
  border-radius: 10px;
  background: #0F172A;
  padding: 0 14px;
  color: #E2E8F0;
  transition: all 0.2s;
}

:deep(.field-pw) {
  width: 100%;
}

.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: #22D3EE;
  background: #0F172A;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3);
  outline: none;
}

.field-box :deep(.field-input::placeholder),
:deep(.field-pw .field-input::placeholder) {
  color: #475569;
}

:deep(.p-password-toggle-mask-icon),
:deep(.p-password .p-icon) {
  color: #64748B;
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
  color: #f87171;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
}

.msg-ok {
  color: #22D3EE;
  background: rgba(34, 211, 238, 0.1);
  border: 1px solid rgba(34, 211, 238, 0.25);
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
  height: 42px;
  font-size: 0.95rem;
  font-weight: 600;
  border-radius: 10px;
  margin-top: 2px;
  background: transparent !important;
  border: 1px solid #22D3EE !important;
  color: #22D3EE !important;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3);
  transition: all 0.25s;
}

.submit-btn:hover {
  background: rgba(34, 211, 238, 0.15) !important;
  box-shadow: 0 0 20px rgba(34, 211, 238, 0.5);
  transform: translateY(-1px);
}

.bottom-links {
  margin-top: 16px;
  padding: 10px 0 0;
  display: flex;
  justify-content: center;
  gap: 8px;
  border-top: 1px solid #334155;
}

.link-text {
  font-size: 0.85rem;
  color: #22D3EE;
  text-decoration: none;
  font-weight: 500;
  transition: all 0.15s;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.link-text:hover {
  color: #67E8F9;
  text-shadow: 0 0 8px rgba(34, 211, 238, 0.4);
}

.bottom-sep {
  color: #334155;
  font-size: 0.8rem;
}
</style>
