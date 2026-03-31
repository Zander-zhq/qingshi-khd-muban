<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../../components/TitleBar.vue'
import { userRegisterApi, sendSmsCodeApi } from '../../api/auth'
import { getDeviceId } from '../../utils/device'
import { getAppCredentials } from '../../utils/config'

const router = useRouter()

const phone = ref('')
const password = ref('')
const confirmPassword = ref('')
const email = ref('')
const smsCode = ref('')
const inviteCode = ref('')
const deviceId = ref('')
const loading = ref(false)
const smsLoading = ref(false)
const smsCooldown = ref(0)
const errMsg = ref('')
const successMsg = ref('')

let cooldownTimer: ReturnType<typeof setInterval> | null = null

const canSendSms = computed(() => {
  return /^1[3-9]\d{9}$/.test(phone.value.trim()) && smsCooldown.value === 0
})

const appId = ref('')

onMounted(async () => {
  const [did, creds] = await Promise.all([getDeviceId(), getAppCredentials()])
  deviceId.value = did
  appId.value = creds.appId
})

function clearMsg() {
  errMsg.value = ''
  successMsg.value = ''
}

async function handleSendSms() {
  if (!canSendSms.value) return
  clearMsg()

  smsLoading.value = true
  try {
    await sendSmsCodeApi({ phone: phone.value.trim() })
    successMsg.value = '验证码已发送'
    smsCooldown.value = 60
    cooldownTimer = setInterval(() => {
      smsCooldown.value--
      if (smsCooldown.value <= 0) {
        clearInterval(cooldownTimer!)
        cooldownTimer = null
      }
    }, 1000)
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '发送失败'
  } finally {
    smsLoading.value = false
  }
}

async function handleRegister() {
  clearMsg()

  const phoneVal = phone.value.trim()
  if (!phoneVal) { errMsg.value = '请输入手机号'; return }
  if (!/^1[3-9]\d{9}$/.test(phoneVal)) { errMsg.value = '手机号格式不正确'; return }
  if (!password.value || password.value.length < 6) { errMsg.value = '密码至少6位'; return }
  if (password.value.length > 18) { errMsg.value = '密码最长18位'; return }
  if (password.value !== confirmPassword.value) { errMsg.value = '两次密码不一致'; return }
  if (!email.value.trim()) { errMsg.value = '请输入邮箱'; return }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value.trim())) { errMsg.value = '邮箱格式不正确'; return }

  loading.value = true
  try {
    await userRegisterApi({
      app_id: appId.value,
      phone: phoneVal,
      password: password.value,
      device_id: deviceId.value,
      acctno: '',
      sms_code: smsCode.value.trim() || '',
      nickname: '',
      invite_code: inviteCode.value.trim() || '',
      email: email.value.trim() || '',
    })
    successMsg.value = '注册成功，正在跳转...'
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '注册失败'
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
        <div class="banner-title">注册账号</div>
      </div>

      <div class="body">
        <form class="form" @submit.prevent="handleRegister">
          <div class="sms-row">
            <div class="field-box sms-field">
              <InputText
                v-model="phone"
                placeholder="手机号"
                class="field-input"
                maxlength="11"
                @input="clearMsg"
              />
            </div>
            <Button
              type="button"
              :label="smsCooldown > 0 ? `${smsCooldown}s` : '验证码'"
              :disabled="!canSendSms"
              :loading="smsLoading"
              size="small"
              class="sms-btn"
              @click="handleSendSms"
            />
          </div>

          <div class="field-box">
            <InputText
              v-model="smsCode"
              placeholder="短信验证码"
              class="field-input"
              maxlength="6"
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
              v-model="email"
              placeholder="邮箱（用于找回密码）"
              class="field-input"
              type="email"
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
  height: 90px;
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
  padding: 16px 40px 16px;
  overflow-y: auto;
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

.sms-row {
  display: flex;
  gap: 10px;
  align-items: stretch;
}

.sms-field {
  flex: 1;
}

.sms-btn {
  white-space: nowrap;
  min-width: 80px;
  font-size: 0.82rem;
  border-radius: 10px;
  height: 42px;
  background: var(--qs-primary) !important;
  border-color: var(--qs-primary) !important;
  color: #fff !important;
}

.sms-btn:hover {
  background: var(--qs-primary-dark) !important;
  border-color: var(--qs-primary-dark) !important;
}

.device-info {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: #94a3b8;
  padding: 4px 0;
  user-select: text;
}

.device-info i {
  font-size: 0.72rem;
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
  height: 42px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 21px;
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
  padding: 10px 0 12px;
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
