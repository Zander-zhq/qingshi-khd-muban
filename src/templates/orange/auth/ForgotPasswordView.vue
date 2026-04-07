<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { sendEmailCodeApi, resetPasswordApi } from '../../../api/auth'
import { getAppCredentials } from '../../../utils/config'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'

const brand = getBrand()
const brandLogo = getBrandLogo()
const router = useRouter()
const appId = ref('')

onMounted(async () => { const creds = await getAppCredentials(); appId.value = creds.appId })

const step = ref<'email' | 'reset'>('email')
const email = ref(''); const code = ref(''); const newPassword = ref(''); const confirmPassword = ref('')
const loading = ref(false); const sendingCode = ref(false); const codeCooldown = ref(0)
const errMsg = ref(''); const successMsg = ref('')
let cooldownTimer: ReturnType<typeof setInterval> | null = null

const codeBtnText = computed(() => codeCooldown.value > 0 ? `${codeCooldown.value}s` : '发送验证码')

function startCooldown() {
  codeCooldown.value = 60
  cooldownTimer = setInterval(() => { codeCooldown.value--; if (codeCooldown.value <= 0 && cooldownTimer) { clearInterval(cooldownTimer); cooldownTimer = null } }, 1000)
}

onUnmounted(() => { if (cooldownTimer) clearInterval(cooldownTimer) })
function clearMsg() { errMsg.value = ''; successMsg.value = '' }

async function handleSendCode() {
  clearMsg()
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value.trim())) { errMsg.value = '请输入正确的邮箱地址'; return }
  sendingCode.value = true
  try { await sendEmailCodeApi({ app_id: appId.value, email: email.value.trim(), scene: 'reset_password' }); successMsg.value = '验证码已发送到邮箱'; startCooldown() }
  catch (err: unknown) { errMsg.value = err instanceof Error ? err.message : '发送失败' }
  finally { sendingCode.value = false }
}

function handleNextStep() { clearMsg(); if (!email.value.trim()) { errMsg.value = '请输入邮箱地址'; return }; if (!code.value.trim()) { errMsg.value = '请输入验证码'; return }; step.value = 'reset' }

async function handleResetPassword() {
  clearMsg()
  if (!newPassword.value || newPassword.value.length < 6) { errMsg.value = '密码至少6位'; return }
  if (newPassword.value !== confirmPassword.value) { errMsg.value = '两次密码不一致'; return }
  loading.value = true
  try { await resetPasswordApi({ app_id: appId.value, email: email.value.trim(), code: code.value.trim(), new_password: newPassword.value }); successMsg.value = '密码重置成功，正在跳转...'; setTimeout(() => router.push('/login'), 1500) }
  catch (err: unknown) { errMsg.value = err instanceof Error ? err.message : '重置失败' }
  finally { loading.value = false }
}
</script>

<template>
  <div class="page-shell">
    <TitleBar variant="auth" />
    <div class="page-split">
      <div class="left-panel">
        <div class="lp-deco lp-deco-1"></div>
        <div class="lp-deco lp-deco-2"></div>
        <div class="lp-content">
          <img :src="brandLogo" alt="" class="lp-logo" />
          <h1 class="lp-title">{{ brand.brand_name }}</h1>
          <p class="lp-sub">{{ brand.product_name }}</p>
          <div class="lp-divider"></div>
          <p class="lp-hint">通过邮箱验证重置密码</p>
        </div>
        <div class="lp-ver">{{ VERSION }}</div>
      </div>

      <div class="right-panel">
        <div class="rp-content">
          <h2 class="rp-title">找回密码</h2>
          <p class="rp-desc">{{ step === 'email' ? '请输入注册邮箱' : '设置新密码' }}</p>

          <template v-if="step === 'email'">
            <form class="form" @submit.prevent="handleNextStep">
              <div class="field"><div class="input-box"><i class="pi pi-envelope input-icon"></i><InputText v-model="email" placeholder="邮箱地址" class="gk-input" type="email" @input="clearMsg" /></div></div>
              <div class="field code-row">
                <div class="input-box" style="flex:1"><i class="pi pi-key input-icon"></i><InputText v-model="code" placeholder="验证码" class="gk-input" maxlength="6" @input="clearMsg" /></div>
                <Button type="button" :label="codeBtnText" :disabled="codeCooldown > 0" :loading="sendingCode" class="code-btn" @click="handleSendCode" />
              </div>
              <Transition name="fade">
                <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
                <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
              </Transition>
              <Button type="submit" label="下一步" class="submit-btn" />
            </form>
          </template>

          <template v-else>
            <form class="form" @submit.prevent="handleResetPassword">
              <div class="field"><div class="input-box"><i class="pi pi-lock input-icon"></i><Password v-model="newPassword" placeholder="新密码（至少6位）" :feedback="false" toggleMask class="gk-pw" inputClass="gk-input" @input="clearMsg" /></div></div>
              <div class="field"><div class="input-box"><i class="pi pi-lock input-icon"></i><Password v-model="confirmPassword" placeholder="确认新密码" :feedback="false" toggleMask class="gk-pw" inputClass="gk-input" @input="clearMsg" /></div></div>
              <Transition name="fade">
                <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
                <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
              </Transition>
              <Button type="submit" label="重置密码" :loading="loading" class="submit-btn" />
              <button type="button" class="back-link" @click="step = 'email'; clearMsg()">返回上一步</button>
            </form>
          </template>

          <div class="rp-links">
            <a href="#" @click.prevent="router.push('/login')"><i class="pi pi-arrow-left"></i> 返回登录</a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import './auth-shared.css';
</style>
