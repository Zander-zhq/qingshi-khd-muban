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

      <div class="body">
        <div class="form-card">
          <h2 class="card-title">解绑设备</h2>

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
.window-shell { height: 100vh; width: 100vw; background: #0F172A; }
.window-content { height: 100%; width: 100%; display: flex; flex-direction: column; background: #0F172A; overflow-y: auto; }

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
  margin: 0 0 20px;
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

.form { width: 100%; display: flex; flex-direction: column; gap: 14px; }
.field-box { width: 100%; }

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%; height: 44px; font-size: 0.92rem;
  border: 1px solid #334155; border-radius: 10px; background: #0F172A; padding: 0 16px;
  color: #E2E8F0; transition: all 0.2s;
}

:deep(.field-pw) { width: 100%; }

.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: #22D3EE; background: #0F172A;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3); outline: none;
}

.field-box :deep(.field-input::placeholder),
:deep(.field-pw .field-input::placeholder) { color: #475569; }

:deep(.p-password-toggle-mask-icon),
:deep(.p-password .p-icon) { color: #64748B; }

.msg-tip { display: flex; align-items: center; gap: 6px; font-size: 0.82rem; padding: 8px 12px; border-radius: 8px; width: 100%; }
.msg-err { color: #f87171; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.25); }
.msg-ok { color: #22D3EE; background: rgba(34, 211, 238, 0.1); border: 1px solid rgba(34, 211, 238, 0.25); }

.fade-enter-active, .fade-leave-active { transition: all 0.25s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }

.submit-btn {
  width: 100%; height: 44px; font-size: 1rem; font-weight: 600; border-radius: 10px; margin-top: 4px;
  background: transparent !important; border: 1px solid #22D3EE !important; color: #22D3EE !important;
  box-shadow: 0 0 12px rgba(34, 211, 238, 0.3); transition: all 0.25s;
}
.submit-btn:hover {
  background: rgba(34, 211, 238, 0.15) !important;
  box-shadow: 0 0 20px rgba(34, 211, 238, 0.5); transform: translateY(-1px);
}

.bottom-links {
  flex-shrink: 0; padding: 16px 0 0; display: flex; justify-content: center;
  border-top: 1px solid #334155; margin-top: 16px;
}
.link-text {
  font-size: 0.85rem; color: #22D3EE; text-decoration: none; font-weight: 500;
  transition: all 0.15s; display: inline-flex; align-items: center; gap: 4px;
}
.link-text:hover { color: #67E8F9; text-shadow: 0 0 8px rgba(34, 211, 238, 0.4); }

.device-list-title { font-size: 0.88rem; color: #94A3B8; margin-bottom: 12px; text-align: center; }

.device-list { width: 100%; display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; max-height: 240px; overflow-y: auto; }

.device-item {
  display: flex; align-items: center; gap: 12px; padding: 12px 14px;
  border: 1px solid #334155; border-radius: 10px; cursor: pointer; transition: all 0.15s;
}
.device-item:hover { border-color: rgba(34, 211, 238, 0.4); background: rgba(34, 211, 238, 0.05); }
.device-item--selected { border-color: #22D3EE; background: rgba(34, 211, 238, 0.08); box-shadow: 0 0 8px rgba(34, 211, 238, 0.2); }

.device-radio { accent-color: #22D3EE; width: 16px; height: 16px; flex-shrink: 0; }

.device-info { flex: 1; min-width: 0; }
.device-sn { font-size: 0.88rem; font-weight: 600; color: #E2E8F0; word-break: break-all; }
.device-time { font-size: 0.75rem; color: #64748B; margin-top: 2px; }

.back-link {
  margin-top: 10px; border: none; background: none; color: #64748B; font-size: 0.82rem;
  cursor: pointer; text-decoration: underline; transition: color 0.15s;
}
.back-link:hover { color: #22D3EE; }
</style>
