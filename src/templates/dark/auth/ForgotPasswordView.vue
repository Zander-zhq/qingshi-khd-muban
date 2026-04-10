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

      <div class="scroll-body">
      <div class="body">
        <div class="form-card">
          <h2 class="card-title">找回密码</h2>

          <div class="form-area">
            <template v-if="step === 'email'">
              <p class="step-hint">请输入邮箱用来找回密码</p>
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
              <p class="step-hint">设置新密码</p>
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

.form-area {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.step-hint {
  font-size: 0.88rem;
  color: #94A3B8;
  text-align: center;
  margin: 8px 0 20px;
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
  height: 44px;
  font-size: 0.92rem;
  border: 1px solid #334155;
  border-radius: 10px;
  background: #0F172A;
  padding: 0 16px;
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
  font-size: 0.82rem;
  padding: 8px 12px;
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
  height: 44px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 10px;
  margin-top: 4px;
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

.resend-wrap {
  text-align: center;
}

.resend-link {
  background: none;
  border: none;
  color: #64748B;
  font-size: 0.82rem;
  cursor: pointer;
  text-decoration: underline;
  transition: color 0.15s;
}

.resend-link:hover {
  color: #22D3EE;
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
  height: 44px;
  font-size: 0.82rem;
  font-weight: 600;
  border-radius: 10px;
  white-space: nowrap;
  padding: 0 14px;
  background: transparent !important;
  border: 1px solid #22D3EE !important;
  color: #22D3EE !important;
  box-shadow: 0 0 8px rgba(34, 211, 238, 0.2);
  transition: all 0.2s;
}

.code-btn:hover {
  background: rgba(34, 211, 238, 0.1) !important;
}

.code-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.bottom-links {
  flex-shrink: 0;
  padding: 16px 0 0;
  display: flex;
  justify-content: center;
  border-top: 1px solid #334155;
  margin-top: 16px;
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
</style>
