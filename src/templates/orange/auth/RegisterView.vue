<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useRegister } from '../../../composables/useRegister'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'

const brand = getBrand()
const brandLogo = getBrandLogo()

const {
  router, phone, password, confirmPassword, inviteCode,
  loading, errMsg, successMsg, clearMsg, handleRegister,
} = useRegister()
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
          <p class="lp-hint">注册后即可体验全部功能</p>
        </div>
        <div class="lp-ver">{{ VERSION }}</div>
      </div>

      <div class="right-panel">
        <div class="rp-content">
          <h2 class="rp-title">注册账号</h2>
          <p class="rp-desc">请填写以下信息完成注册</p>

          <form class="form" @submit.prevent="handleRegister">
            <div class="field"><div class="input-box"><i class="pi pi-phone input-icon"></i><InputText v-model="phone" placeholder="手机号" class="gk-input" maxlength="11" @input="clearMsg" /></div></div>
            <div class="field"><div class="input-box"><i class="pi pi-lock input-icon"></i><Password v-model="password" placeholder="密码（6-18位）" toggleMask class="gk-pw" inputClass="gk-input" autocomplete="new-password" @input="clearMsg" /></div></div>
            <div class="field"><div class="input-box"><i class="pi pi-lock input-icon"></i><Password v-model="confirmPassword" placeholder="确认密码" :feedback="false" toggleMask class="gk-pw" inputClass="gk-input" autocomplete="new-password" @input="clearMsg" /></div></div>
            <div class="field"><div class="input-box"><i class="pi pi-tag input-icon"></i><InputText v-model="inviteCode" placeholder="邀请码（选填）" class="gk-input" @input="clearMsg" /></div></div>

            <Transition name="fade">
              <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
              <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
            </Transition>

            <Button type="submit" label="注 册" :loading="loading" class="submit-btn" />
          </form>

          <div class="rp-links">
            <a href="#" @click.prevent="router.push('/login')"><i class="pi pi-arrow-left"></i> 返回登录</a>
            <a href="#" @click.prevent="router.push('/recharge')"><i class="pi pi-credit-card"></i> 卡密充值</a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import './auth-shared.css';
</style>
