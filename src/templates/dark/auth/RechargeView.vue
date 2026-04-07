<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { redeemCardApi } from '../../../api/auth'
import { getAppCredentials } from '../../../utils/config'

const router = useRouter()
const appId = ref('')

onMounted(async () => {
  const creds = await getAppCredentials()
  appId.value = creds.appId
})

const acctno = ref('')
const cardKey = ref('')
const loading = ref(false)
const errMsg = ref('')
const successMsg = ref('')

function clearMsg() {
  errMsg.value = ''
  successMsg.value = ''
}

async function handleRecharge() {
  clearMsg()

  if (!acctno.value.trim()) { errMsg.value = '请输入账号（手机号）'; return }
  if (!cardKey.value.trim()) { errMsg.value = '请输入卡密'; return }

  loading.value = true
  try {
    const res = await redeemCardApi({
      app_id: appId.value,
      acctno: acctno.value.trim(),
      card_key: cardKey.value.trim(),
    })
    const cardType = (res as any).card_type || ''
    const expireAt = (res as any).vip_expire_at || ''
    let msg = '充值成功！'
    if (cardType) msg += ` (${cardType})`
    if (expireAt) {
      const d = new Date(expireAt)
      msg += `  到期：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    }
    successMsg.value = msg
    cardKey.value = ''
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) {
    errMsg.value = err instanceof Error ? err.message : '充值失败'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />

      <div class="body">
        <div class="form-card">
          <h2 class="card-title">卡密充值</h2>
          <p class="card-subtitle">输入卡密为账号充值</p>

          <div class="form-area">
            <form class="form" @submit.prevent="handleRecharge">
              <div class="field-box">
                <InputText
                  v-model="acctno"
                  placeholder="账号（手机号）"
                  class="field-input"
                  @input="clearMsg"
                />
              </div>

              <div class="field-box">
                <InputText
                  v-model="cardKey"
                  placeholder="卡密"
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

              <Button type="submit" label="充 值" :loading="loading" class="submit-btn" />
            </form>
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
  overflow-y: auto;
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

.form-area {
  display: flex;
  flex-direction: column;
  align-items: center;
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

.field-box :deep(.field-input) {
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

.field-box :deep(.field-input:focus) {
  border-color: #22D3EE;
  background: #0F172A;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3);
  outline: none;
}

.field-box :deep(.field-input::placeholder) {
  color: #475569;
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
