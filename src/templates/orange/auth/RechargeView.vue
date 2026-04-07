<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { redeemCardApi } from '../../../api/auth'
import { getAppCredentials } from '../../../utils/config'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'

const brand = getBrand()
const brandLogo = getBrandLogo()
const router = useRouter()
const appId = ref('')

onMounted(async () => { const creds = await getAppCredentials(); appId.value = creds.appId })

const acctno = ref(''); const cardKey = ref('')
const loading = ref(false); const errMsg = ref(''); const successMsg = ref('')

function clearMsg() { errMsg.value = ''; successMsg.value = '' }

async function handleRecharge() {
  clearMsg()
  if (!acctno.value.trim()) { errMsg.value = '请输入账号（手机号）'; return }
  if (!cardKey.value.trim()) { errMsg.value = '请输入卡密'; return }

  loading.value = true
  try {
    const res = await redeemCardApi({ app_id: appId.value, acctno: acctno.value.trim(), card_key: cardKey.value.trim() })
    const cardType = (res as any).card_type || ''; const expireAt = (res as any).vip_expire_at || ''
    let msg = '充值成功！'
    if (cardType) msg += ` (${cardType})`
    if (expireAt) { const d = new Date(expireAt); msg += `  到期：${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}` }
    successMsg.value = msg; cardKey.value = ''
    setTimeout(() => router.push('/login'), 1500)
  } catch (err: unknown) { errMsg.value = err instanceof Error ? err.message : '充值失败' }
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
          <p class="lp-hint">输入卡密即可充值会员</p>
        </div>
        <div class="lp-ver">{{ VERSION }}</div>
      </div>

      <div class="right-panel">
        <div class="rp-content">
          <h2 class="rp-title">卡密充值</h2>
          <p class="rp-desc">请输入账号和卡密进行充值</p>

          <form class="form" @submit.prevent="handleRecharge">
            <div class="field"><div class="input-box"><i class="pi pi-user input-icon"></i><InputText v-model="acctno" placeholder="账号（手机号）" class="gk-input" @input="clearMsg" /></div></div>
            <div class="field"><div class="input-box"><i class="pi pi-credit-card input-icon"></i><InputText v-model="cardKey" placeholder="卡密" class="gk-input" @input="clearMsg" /></div></div>
            <Transition name="fade">
              <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
              <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
            </Transition>
            <Button type="submit" label="充 值" :loading="loading" class="submit-btn" />
          </form>

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
