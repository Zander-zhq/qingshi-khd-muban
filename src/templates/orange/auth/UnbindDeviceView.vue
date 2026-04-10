<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useUnbindDevicePage } from '../../../composables/useUnbindDevicePage'
import { getBrand, getBrandLogo, VERSION } from '../../../brand'

const brand = getBrand()
const brandLogo = getBrandLogo()

const {
  router, acctno, password, loading, errMsg, successMsg,
  showDeviceList, devices, selectedDevice,
  clearMsg, handleSubmit, handleSelectDevice,
} = useUnbindDevicePage()
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
          <p class="lp-hint">解绑设备后可在新设备登录</p>
        </div>
        <div class="lp-ver">{{ VERSION }}</div>
      </div>

      <div class="right-panel">
        <div class="rp-content">
          <h2 class="rp-title">解绑设备</h2>
          <p class="rp-desc">请输入账号密码验证身份</p>

          <template v-if="!showDeviceList">
            <form class="form" @submit.prevent="handleSubmit">
              <div class="field"><div class="input-box"><i class="pi pi-user input-icon"></i><InputText v-model="acctno" placeholder="账号（手机号）" class="gk-input" @input="clearMsg" /></div></div>
              <div class="field"><div class="input-box"><i class="pi pi-lock input-icon"></i><Password v-model="password" placeholder="密码" :feedback="false" toggleMask class="gk-pw" inputClass="gk-input" @input="clearMsg" /></div></div>
              <Transition name="fade">
                <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
                <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
              </Transition>
              <Button type="submit" label="解绑设备" :loading="loading" class="submit-btn" />
            </form>
          </template>

          <template v-else>
            <div class="device-title">请选择要解绑的设备：</div>
            <div class="device-list">
              <label v-for="d in devices" :key="d.device_sn" class="device-row" :class="{ 'device-row--on': selectedDevice === d.device_sn }">
                <input type="radio" v-model="selectedDevice" :value="d.device_sn" class="device-radio" />
                <div class="device-info"><div class="device-sn">{{ d.device_sn }}</div><div class="device-time">最后登录：{{ d.last_login }}</div></div>
              </label>
            </div>
            <Transition name="fade">
              <div v-if="errMsg" class="msg msg--err"><i class="pi pi-exclamation-circle"></i>{{ errMsg }}</div>
              <div v-else-if="successMsg" class="msg msg--ok"><i class="pi pi-check-circle"></i>{{ successMsg }}</div>
            </Transition>
            <Button label="确认解绑" :loading="loading" class="submit-btn" @click="handleSelectDevice" />
            <button type="button" class="back-link" @click="showDeviceList = false; clearMsg()">返回重新输入</button>
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

.device-title { font-size: 0.82rem; color: #606266; margin-bottom: 10px; }
.device-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; max-height: 200px; overflow-y: auto; }
.device-row { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border: 1px solid #DCDFE6; border-radius: 4px; cursor: pointer; transition: all 0.15s; }
.device-row:hover { border-color: #F97316; background: #FFF7ED; }
.device-row--on { border-color: #F97316; background: #FFF7ED; }
.device-radio { accent-color: #F97316; width: 14px; height: 14px; flex-shrink: 0; }
.device-info { flex: 1; min-width: 0; }
.device-sn { font-size: 0.82rem; font-weight: 600; color: #303133; word-break: break-all; }
.device-time { font-size: 0.68rem; color: #909399; margin-top: 2px; }
</style>
