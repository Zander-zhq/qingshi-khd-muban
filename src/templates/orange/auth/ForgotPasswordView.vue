<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useForgotPassword } from '../../../composables/useForgotPassword'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'

const brand = getBrand()
const brandLogo = getBrandLogo()

const {
  router, step, email, code, newPassword, confirmPassword,
  loading, sendingCode, codeCooldown, errMsg, successMsg,
  codeBtnText, clearMsg, handleSendCode, handleNextStep, handleResetPassword,
} = useForgotPassword()
</script>

<template>
  <div class="page-shell">
    <TitleBar variant="auth" />
    <div class="scroll-body">
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
  </div>
</template>

<style scoped>
@import './auth-shared.css';
</style>
