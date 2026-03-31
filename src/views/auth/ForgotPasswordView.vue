<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../../components/TitleBar.vue'
async function forgotPasswordApi(_data: { email: string }) {
  throw new Error('找回密码功能暂未开放')
}
async function resetPasswordApi(_data: { email: string; code: string; new_password: string }) {
  throw new Error('找回密码功能暂未开放')
}

const router = useRouter()

const step = ref<'email' | 'reset'>('email')
const email = ref('')
const code = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const loading = ref(false)
const errMsg = ref('')
const successMsg = ref('')

function clearMsg() {
  errMsg.value = ''
  successMsg.value = ''
}

async function handleSendCode() {
  clearMsg()
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  if (!emailRegex.test(email.value.trim())) {
    errMsg.value = '请输入正确的邮箱地址'
    return
  }

  loading.value = true
  try {
    await forgotPasswordApi({ email: email.value.trim() })
    successMsg.value = '验证码已发送到邮箱'
    step.value = 'reset'
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '发送失败'
  } finally {
    loading.value = false
  }
}

async function handleResetPassword() {
  clearMsg()
  if (!code.value.trim()) { errMsg.value = '请输入验证码'; return }
  if (!newPassword.value || newPassword.value.length < 6) { errMsg.value = '密码至少6位'; return }
  if (newPassword.value !== confirmPassword.value) { errMsg.value = '两次密码不一致'; return }

  loading.value = true
  try {
    await resetPasswordApi({
      email: email.value.trim(),
      code: code.value.trim(),
      new_password: newPassword.value,
    })
    successMsg.value = '密码重置成功，正在跳转...'
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '重置失败'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar />

      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="banner-title">找回密码</div>
      </div>

      <div class="body">
        <template v-if="step === 'email'">
          <div class="step-hint">请输入注册时使用的邮箱</div>
          <form class="form" @submit.prevent="handleSendCode">
            <div class="field-box">
              <InputText v-model="email" placeholder="邮箱地址" class="field-input" type="email" @input="clearMsg" />
            </div>

            <Transition name="fade">
              <div v-if="errMsg" class="msg-tip msg-err">
                <i class="pi pi-exclamation-circle"></i>{{ errMsg }}
              </div>
            </Transition>

            <Button type="submit" label="发送验证码" :loading="loading" class="submit-btn" />
          </form>
        </template>

        <template v-else>
          <div class="step-hint">验证码已发送至 {{ email }}</div>
          <form class="form" @submit.prevent="handleResetPassword">
            <div class="field-box">
              <InputText v-model="code" placeholder="邮箱验证码" class="field-input" maxlength="6" @input="clearMsg" />
            </div>

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
              <button type="button" class="resend-link" @click="step = 'email'; clearMsg()">重新发送验证码</button>
            </div>
          </form>
        </template>

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
  overflow: hidden;
  position: relative;
}

.banner {
  height: 100px;
  position: relative;
  background: var(--qs-bg-gradient);
  flex-shrink: 0;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  padding-top: 10px;
}

.bc {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.07);
}
.bc-1 { width: 160px; height: 160px; top: -60px; right: -20px; }
.bc-2 { width: 90px; height: 90px; bottom: -30px; left: 10px; }

.banner-title {
  position: relative;
  z-index: 1;
  font-size: 1.5rem;
  font-weight: 600;
  color: #fff;
  letter-spacing: 0.1em;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.1);
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 28px 40px 24px;
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

.bottom-links {
  margin-top: auto;
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
