<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import TitleBar from '../TitleBar.vue'
import { useUnbindDevicePage } from '../../../composables/useUnbindDevicePage'

const {
  router, acctno, password, loading, errMsg, successMsg,
  showDeviceList, devices, selectedDevice,
  clearMsg, handleSubmit, handleSelectDevice,
} = useUnbindDevicePage()
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="scroll-body">
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="banner-title">解绑设备</div>
      </div>

      <div class="body">
        <div class="form-area">
          <template v-if="!showDeviceList">
            <form class="form" @submit.prevent="handleSubmit">
              <div class="field-box">
                <InputText v-model="acctno" placeholder="账号（手机号）" class="field-input" @input="clearMsg" />
              </div>
              <div class="field-box">
                <Password v-model="password" placeholder="密码" :feedback="false" toggleMask class="field-pw" inputClass="field-input" @input="clearMsg" />
              </div>

              <Transition name="fade">
                <div v-if="errMsg" class="msg-tip msg-err">
                  <i class="pi pi-exclamation-circle"></i>{{ errMsg }}
                </div>
                <div v-else-if="successMsg" class="msg-tip msg-ok">
                  <i class="pi pi-check-circle"></i>{{ successMsg }}
                </div>
              </Transition>

              <Button type="submit" label="解绑设备" :loading="loading" class="submit-btn" />
            </form>
          </template>

          <template v-else>
            <div class="device-list-title">该账号绑定了多台设备，请选择要解绑的：</div>
            <div class="device-list">
              <label
                v-for="d in devices"
                :key="d.device_sn"
                class="device-item"
                :class="{ 'device-item--selected': selectedDevice === d.device_sn }"
              >
                <input type="radio" v-model="selectedDevice" :value="d.device_sn" class="device-radio" />
                <div class="device-info">
                  <div class="device-sn">{{ d.device_sn }}</div>
                  <div class="device-time">最后登录：{{ d.last_login }}</div>
                </div>
              </label>
            </div>

            <Transition name="fade">
              <div v-if="errMsg" class="msg-tip msg-err">
                <i class="pi pi-exclamation-circle"></i>{{ errMsg }}
              </div>
              <div v-else-if="successMsg" class="msg-tip msg-ok">
                <i class="pi pi-check-circle"></i>{{ successMsg }}
              </div>
            </Transition>

            <Button label="确认解绑" :loading="loading" class="submit-btn" @click="handleSelectDevice" />
            <button type="button" class="back-link" @click="showDeviceList = false; clearMsg()">返回重新输入</button>
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
</template>

<style scoped>
.window-shell { height: 100vh; width: 100vw; background: #fff; }
.window-content { height: 100%; width: 100%; display: flex; flex-direction: column; background: #fff; overflow: hidden; }
.window-content :deep(.app-titlebar) { flex-shrink: 0; }
.scroll-body { flex: 1; overflow-y: auto; display: flex; flex-direction: column; }
.window-content :deep(.app-titlebar.titlebar-compact) { position: relative; }

.banner { height: 100px; position: relative; background: var(--qs-bg-gradient); flex-shrink: 0; overflow: visible; display: flex; align-items: center; justify-content: center; }
.bc { position: absolute; border-radius: 50%; background: rgba(255, 255, 255, 0.07); pointer-events: none; }
.bc-1 { width: 140px; height: 140px; top: -50px; right: -20px; }
.bc-2 { width: 80px; height: 80px; bottom: -30px; left: 10px; }
.banner-title { position: relative; z-index: 1; font-size: 1.4rem; font-weight: 700; color: #fff; letter-spacing: 0.12em; text-shadow: 0 1px 8px rgba(0, 0, 0, 0.1); }

.body { flex: 1; display: flex; flex-direction: column; padding: 0 36px; min-height: 0; }
.form-area { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; }

.form { width: 100%; display: flex; flex-direction: column; gap: 14px; }
.field-box { width: 100%; }

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%; height: 44px; font-size: 0.92rem;
  border: 1.5px solid #e2e8f0; border-radius: 10px; background: #f8fafb; padding: 0 16px; transition: all 0.2s;
}
:deep(.field-pw) { width: 100%; }
.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) { border-color: var(--qs-primary-light); background: #fff; box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.08); }

.msg-tip { display: flex; align-items: center; gap: 6px; font-size: 0.82rem; padding: 8px 12px; border-radius: 8px; width: 100%; }
.msg-err { color: #dc2626; background: #fef2f2; border: 1px solid #fecaca; }
.msg-ok { color: var(--qs-primary-dark); background: #f0fdfa; border: 1px solid #99f6e4; }

.fade-enter-active, .fade-leave-active { transition: all 0.25s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }

.submit-btn {
  width: 100%; height: 44px; font-size: 1rem; font-weight: 600; border-radius: 22px; margin-top: 4px;
  background: var(--qs-bg-gradient) !important; border: none !important;
  box-shadow: 0 4px 16px rgba(13, 148, 136, 0.3); transition: all 0.2s;
}
.submit-btn:hover { box-shadow: 0 6px 24px rgba(13, 148, 136, 0.4); transform: translateY(-1px); }

.bottom-links { flex-shrink: 0; padding: 16px 0 14px; display: flex; justify-content: center; }
.link-text { font-size: 0.85rem; color: var(--qs-primary); text-decoration: none; font-weight: 500; transition: color 0.15s; display: inline-flex; align-items: center; gap: 4px; }
.link-text:hover { color: var(--qs-primary-dark); }

.device-list-title { font-size: 0.88rem; color: #475569; margin-bottom: 12px; text-align: center; }

.device-list { width: 100%; display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; max-height: 240px; overflow-y: auto; }

.device-item {
  display: flex; align-items: center; gap: 12px; padding: 12px 14px;
  border: 1.5px solid #e2e8f0; border-radius: 10px; cursor: pointer; transition: all 0.15s;
}
.device-item:hover { border-color: var(--qs-primary-light, #2dd4bf); background: #f0fdfa; }
.device-item--selected { border-color: var(--qs-primary, #0d9488); background: #f0fdfa; }

.device-radio { accent-color: var(--qs-primary, #0d9488); width: 16px; height: 16px; flex-shrink: 0; }

.device-info { flex: 1; min-width: 0; }
.device-sn { font-size: 0.88rem; font-weight: 600; color: #0f172a; word-break: break-all; }
.device-time { font-size: 0.75rem; color: #94a3b8; margin-top: 2px; }

.back-link {
  margin-top: 10px; border: none; background: none; color: #94a3b8; font-size: 0.82rem;
  cursor: pointer; text-decoration: underline; transition: color 0.15s;
}
.back-link:hover { color: var(--qs-primary); }
</style>
