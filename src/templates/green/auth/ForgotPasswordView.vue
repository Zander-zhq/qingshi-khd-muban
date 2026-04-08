<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useForgotPassword } from '../../../composables/useForgotPassword'

const {
  router, step, email, code, newPassword, confirmPassword,
  loading, sendingCode, codeCooldown, errMsg, successMsg,
  codeBtnText, clearMsg, handleSendCode, handleNextStep, handleResetPassword,
} = useForgotPassword()
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="banner-title">找回密码</div>
      </div>

      <div class="body">
        <div class="form-area">
          <template v-if="step === 'email'">
            <div class="step-hint">请输入邮箱用来找回密码</div>
            <form class="form" @submit.prevent="handleNextStep">
              <div class="field-box">
                <InputText v-model="email" placeholder="邮箱地址" class="field-input" type="email" @input="clearMsg" />
              </div>

              <div class="field-box code-row">
                <InputText v-model="code" placeholder="验证码" class="field-input code-input" maxlength="6" @input="clearMsg" />
                <Button
                  type="button"
                  :label="codeBtnText"
                  :disabled="codeCooldown > 0"
                  :loading="sendingCode"
                  class="code-btn"
                  @click="handleSendCode"
                />
              </div>

              <Transition name="fade">
                <div v-if="errMsg" class="msg-tip msg-err">
                  <i class="pi pi-exclamation-circle"></i>{{ errMsg }}
                </div>
                <div v-else-if="successMsg" class="msg-tip msg-ok">
                  <i class="pi pi-check-circle"></i>{{ successMsg }}
                </div>
              </Transition>

              <Button type="submit" label="下一步" class="submit-btn" />
            </form>
          </template>

          <template v-else>
            <div class="step-hint">设置新密码</div>
            <form class="form" @submit.prevent="handleResetPassword">
              <div class="field-box">
                <Password v-model="newPassword" placeholder="新密码（至少6位）" :feedback="false" toggleMask class="field-pw" inputClass="field-input" @input="clearMsg" />
              </div>

              <div class="field-box">
                <Password v-model="confirmPassword" placeholder="确认新密码" :feedback="false" toggleMask class="field-pw" inputClass="field-input" @input="clearMsg" />
              </div>

              <Transition name="fade">
                <div v-if="errMsg" class="msg-tip msg-err">
                  <i class="pi pi-exclamation-circle"></i>{{ errMsg }}
                </div>
                <div v-else-if="successMsg" class="msg-tip msg-ok">
                  <i class="pi pi-check-circle"></i>{{ successMsg }}
                </div>
              </Transition>

              <Button type="submit" label="重置密码" :loading="loading" class="submit-btn" />

              <div class="resend-wrap">
                <button type="button" class="resend-link" @click="step = 'email'; clearMsg()">返回上一步</button>
              </div>
            </form>
          </template>
        </div>

        <div class="bottom-links">
          <a href="#" class="link-text" @click.prevent="router.push('/login')">
            <i class="pi pi-arrow-left" style="font-size: 0.7rem"></i>
            返回登录
          </a>
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
  overflow-y: auto;
  position: relative;
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
.bc-1 { width: 160px; height: 160px; top: -60px; right: -20px; }
.bc-2 { width: 90px; height: 90px; bottom: -30px; left: 10px; }

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
  padding: 0 36px;
  min-height: 0;
}

.form-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.step-hint {
  font-size: 0.9rem;
  color: var(--qs-text-secondary);
  text-align: center;
  margin-bottom: 20px;
}

.form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field-box {
  width: 100%;
}

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%;
  height: 46px;
  font-size: 0.92rem;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  background: #f8fafb;
  padding: 0 16px;
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
  font-size: 0.82rem;
  padding: 8px 12px;
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
  height: 46px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 23px;
  margin-top: 4px;
  background: var(--qs-bg-gradient) !important;
  border: none !important;
  box-shadow: 0 4px 16px rgba(13, 148, 136, 0.3);
  transition: all 0.2s;
}

.submit-btn:hover {
  box-shadow: 0 6px 24px rgba(13, 148, 136, 0.4);
  transform: translateY(-1px);
}

.resend-wrap {
  text-align: center;
}

.resend-link {
  background: none;
  border: none;
  color: var(--qs-text-secondary);
  font-size: 0.82rem;
  cursor: pointer;
  text-decoration: underline;
  transition: color 0.15s;
}

.resend-link:hover {
  color: var(--qs-primary);
}

.code-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.code-row :deep(.code-input) {
  flex: 1;
  min-width: 0;
}

.code-btn {
  flex-shrink: 0;
  height: 46px;
  font-size: 0.82rem;
  font-weight: 600;
  border-radius: 10px;
  white-space: nowrap;
  padding: 0 14px;
  background: var(--qs-bg-gradient) !important;
  border: none !important;
  color: #fff !important;
  transition: all 0.2s;
}

.code-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.bottom-links {
  flex-shrink: 0;
  padding: 16px 0 12px;
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
</style>
