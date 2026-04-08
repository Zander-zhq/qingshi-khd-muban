<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import Checkbox from 'primevue/checkbox'
import TitleBar from '../TitleBar.vue'
import { useLogin, getAccountAvatarSrc, maskPhone } from '../../../composables/useLogin'

const {
  router, cachedAvatar, acctno, password, rememberPwd, autoLogin, loading, errMsg,
  autoLoginCountdown, savedAccounts, showDropdown,
  dropdownAccounts, shouldShowDropdown, buttonLabel,
  selectAccount, handleDeleteAccount, onAcctInput, onAcctFocus, onAcctBlur,
  onAcctAreaLeave, cancelAcctAreaLeave, toggleDropdown,
  goRegister, goForgotPassword, handleLogin, handleButtonClick, onPasswordInput,
} = useLogin()
</script>

<template>
  <div class="window-shell">
    <TitleBar variant="auth" />
    <div class="login-bg">
      <div class="glow"></div>
      <div class="login-card">

        <div class="brand-section">
          <div class="brand-icon">
            <i class="pi pi-bolt"></i>
          </div>
          <h2 class="brand-title">登录</h2>
        </div>

        <div class="brand-line"></div>

        <div class="avatar-section">
          <div class="avatar-ring">
            <div class="avatar">
              <img v-if="cachedAvatar" :src="cachedAvatar" class="avatar-img" alt="头像" />
              <i v-else class="pi pi-user"></i>
            </div>
          </div>
        </div>

        <form class="form" @submit.prevent="handleLogin">
          <div class="field-box acct-field" @mouseleave="onAcctAreaLeave" @mouseenter="cancelAcctAreaLeave">
            <div class="acct-input-wrap">
              <InputText
                v-model="acctno"
                placeholder="请输入账号（手机号）"
                class="field-input"
                autocomplete="off"
                @focus="onAcctFocus"
                @blur="onAcctBlur"
                @input="onAcctInput"
              />
              <button
                v-if="savedAccounts.length > 0"
                type="button"
                class="acct-dropdown-toggle"
                tabindex="-1"
                @mousedown.prevent="toggleDropdown"
              >
                <i class="pi" :class="shouldShowDropdown ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
              </button>
            </div>

            <Transition name="dropdown">
              <div v-if="shouldShowDropdown" class="acct-dropdown">
                <div
                  v-for="acct in dropdownAccounts"
                  :key="acct.acctno"
                  class="acct-item"
                  @mousedown.prevent="selectAccount(acct)"
                >
                  <div class="acct-item-avatar">
                    <img v-if="getAccountAvatarSrc(acct)" :src="getAccountAvatarSrc(acct)" class="acct-avatar-img" alt="" />
                    <span v-else>{{ (acct.username || acct.acctno).charAt(0) }}</span>
                  </div>
                  <div class="acct-item-info">
                    <div class="acct-item-name">{{ acct.username || maskPhone(acct.acctno) }}</div>
                    <div class="acct-item-phone">{{ maskPhone(acct.phone || acct.acctno) }}</div>
                  </div>
                  <button
                    type="button"
                    class="acct-item-del"
                    title="删除此账号"
                    @mousedown.prevent.stop="handleDeleteAccount($event, acct.acctno)"
                  >
                    <i class="pi pi-times"></i>
                  </button>
                </div>
                <div v-if="dropdownAccounts.length === 0" class="acct-empty">无匹配账号</div>
              </div>
            </Transition>
          </div>

          <div class="field-box">
            <Password
              v-model="password"
              placeholder="请输入密码"
              :feedback="false"
              toggleMask
              class="field-pw"
              inputClass="field-input"
              autocomplete="current-password"
              @input="onPasswordInput"
            />
          </div>

          <Transition name="fade">
            <div v-if="errMsg" class="err-tip">
              <i class="pi pi-exclamation-circle"></i>
              {{ errMsg }}
            </div>
          </Transition>

          <div class="options-row">
            <div class="option-checks">
              <div class="check-item">
                <Checkbox v-model="rememberPwd" :binary="true" inputId="rememberPwd" />
                <label for="rememberPwd">记住密码</label>
              </div>
              <div class="check-item">
                <Checkbox v-model="autoLogin" :binary="true" inputId="autoLogin" />
                <label for="autoLogin">自动登录</label>
              </div>
            </div>
            <a href="#" class="link-text" @click.prevent="goForgotPassword">找回密码</a>
          </div>

          <Button
            type="button"
            :label="buttonLabel"
            :icon="loading && autoLoginCountdown === 0 ? 'pi pi-spin pi-spinner' : undefined"
            class="submit-btn"
            @click="handleButtonClick"
          />
        </form>

        <div class="bottom-divider"></div>

        <div class="bottom-links">
          <a class="bottom-link" href="#" @click.prevent="goRegister">注册账号</a>
          <span class="bottom-sep">·</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/recharge')">卡密充值</a>
          <span class="bottom-sep">·</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/unbind-device')">解绑设备</a>
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes ring-rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes glow-breathe {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 1; }
}

.window-shell {
  height: 100vh;
  width: 100vw;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  font-family: "Inter", "SF Pro Display", "PingFang SC", system-ui, sans-serif;
  background: #0A0F1E;
}

.login-bg {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(180deg, #0A0F1E 0%, #131B2E 100%);
  position: relative;
  padding: 0 20px;
  animation: page-fade-in 0.4s ease-out;
}

.glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 50% 0%, rgba(34,211,238,0.12) 0%, transparent 60%);
  pointer-events: none;
}

.login-card {
  position: relative;
  width: 100%;
  max-width: 380px;
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(34,211,238,0.15);
  border-radius: 12px;
  padding: 28px 30px 22px;
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 32px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.04);
}

.brand-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.brand-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(135deg, rgba(34,211,238,0.15), rgba(168,85,247,0.15));
  border: 1px solid rgba(34,211,238,0.2);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #22D3EE;
  font-size: 1.1rem;
}

.brand-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: #E2E8F0;
  letter-spacing: 0.08em;
}

.brand-line {
  height: 2px;
  margin: 14px 0;
  border-radius: 1px;
  background: linear-gradient(90deg, #22D3EE, #A855F7, transparent);
}

.avatar-section {
  display: flex;
  justify-content: center;
  margin-bottom: 18px;
}

.avatar-ring {
  position: relative;
  padding: 3px;
  border-radius: 50%;
  background: conic-gradient(#22D3EE, #A855F7, #22D3EE);
  animation: ring-rotate 4s linear infinite;
  box-shadow: 0 0 18px rgba(34,211,238,0.25), 0 0 18px rgba(168,85,247,0.15);
}

.avatar {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: #0A0F1E;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #22D3EE;
  font-size: 1.6rem;
  overflow: hidden;
  border: 2px solid #0A0F1E;
  animation: ring-rotate 4s linear infinite reverse;
}

.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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

.acct-field {
  position: relative;
}

.acct-input-wrap {
  position: relative;
  width: 100%;
}

.acct-input-wrap :deep(.field-input) {
  padding-right: 38px;
}

.acct-dropdown-toggle {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: #475569;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.15s;
  font-size: 0.75rem;
}

.acct-dropdown-toggle:hover {
  background: rgba(34,211,238,0.08);
  color: #22D3EE;
}

.acct-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: #0F1629;
  border: 1px solid rgba(34,211,238,0.12);
  border-radius: 10px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.5);
  z-index: 100;
  max-height: 180px;
  overflow-y: auto;
  padding: 4px;
}

.acct-dropdown::-webkit-scrollbar {
  width: 4px;
}

.acct-dropdown::-webkit-scrollbar-track {
  background: transparent;
}

.acct-dropdown::-webkit-scrollbar-thumb {
  background: rgba(34,211,238,0.2);
  border-radius: 2px;
}

.acct-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s;
}

.acct-item:hover {
  background: rgba(34,211,238,0.06);
}

.acct-item-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: linear-gradient(135deg, #22D3EE, #A855F7);
  color: #0A0F1E;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.82rem;
  font-weight: 600;
  flex-shrink: 0;
  overflow: hidden;
}

.acct-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.acct-item-info {
  flex: 1;
  min-width: 0;
}

.acct-item-name {
  font-size: 0.85rem;
  font-weight: 500;
  color: #E2E8F0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.acct-item-phone {
  font-size: 0.72rem;
  color: #475569;
}

.acct-item-del {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: #334155;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  transition: all 0.15s;
  flex-shrink: 0;
  font-size: 0.7rem;
}

.acct-item-del:hover {
  background: rgba(244,63,94,0.12);
  color: #FB7185;
}

.acct-empty {
  padding: 12px;
  text-align: center;
  color: #475569;
  font-size: 0.82rem;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.field-box :deep(.field-input),
:deep(.field-pw .field-input) {
  width: 100%;
  height: 42px;
  font-size: 0.9rem;
  font-family: inherit;
  border: 1px solid #1E293B;
  border-radius: 8px;
  background: #0A0F1E;
  padding: 0 14px;
  color: #E2E8F0;
  transition: all 0.2s;
}

:deep(.field-pw) {
  width: 100%;
}

.field-box :deep(.field-input:focus),
:deep(.field-pw .field-input:focus) {
  border-color: #22D3EE;
  background: #0A0F1E;
  box-shadow: 0 0 0 3px rgba(34,211,238,0.12), 0 0 12px rgba(34,211,238,0.15);
  outline: none;
}

.field-box :deep(.field-input::placeholder),
:deep(.field-pw .field-input::placeholder) {
  color: #334155;
}

:deep(.p-password-toggle-mask-icon),
:deep(.p-password .p-icon) {
  color: #475569;
}

:deep(.p-password-panel) {
  display: none !important;
}

.err-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  color: #FB7185;
  padding: 8px 12px;
  background: rgba(244,63,94,0.06);
  border-radius: 8px;
  border: 1px solid rgba(244,63,94,0.25);
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

.options-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.option-checks {
  display: flex;
  align-items: center;
  gap: 14px;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 5px;
}

.check-item label {
  font-size: 0.82rem;
  color: #64748B;
  cursor: pointer;
  user-select: none;
  transition: color 0.15s;
}

.check-item label:hover {
  color: #94A3B8;
}

:deep(.p-checkbox .p-checkbox-box) {
  width: 16px;
  height: 16px;
  background: #0A0F1E;
  border: 1px solid #1E293B;
  border-radius: 4px;
  transition: all 0.15s;
}

:deep(.p-checkbox .p-checkbox-box:hover) {
  border-color: #22D3EE;
}

:deep(.p-checkbox .p-checkbox-box.p-highlight) {
  background: linear-gradient(135deg, #22D3EE, #A855F7);
  border-color: transparent;
}

.link-text {
  font-size: 0.82rem;
  color: #22D3EE;
  text-decoration: none;
  font-weight: 500;
  transition: all 0.15s;
}

.link-text:hover {
  color: #A855F7;
}

.submit-btn {
  width: 100%;
  height: 42px;
  font-size: 0.95rem;
  font-weight: 600;
  font-family: inherit;
  border-radius: 8px;
  margin-top: 2px;
  background: linear-gradient(135deg, #22D3EE, #A855F7) !important;
  border: none !important;
  color: #fff !important;
  box-shadow: 0 4px 20px rgba(34,211,238,0.2), 0 4px 20px rgba(168,85,247,0.15);
  transition: all 0.25s;
  letter-spacing: 0.04em;
}

.submit-btn:hover {
  box-shadow: 0 6px 28px rgba(34,211,238,0.35), 0 6px 28px rgba(168,85,247,0.25);
  transform: translateY(-1px);
  filter: brightness(1.1);
}

.submit-btn:active {
  transform: translateY(0);
  filter: brightness(0.95);
}

.bottom-divider {
  height: 1px;
  margin: 16px 0 12px;
  background: linear-gradient(90deg, #22D3EE, #A855F7, transparent);
  opacity: 0.4;
}

.bottom-links {
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.bottom-link {
  display: inline-block;
  padding: 4px 8px;
  font-size: 0.8rem;
  color: #22D3EE;
  text-decoration: none;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.bottom-link:hover {
  color: #A855F7;
}

.bottom-sep {
  color: #1E293B;
  font-size: 0.75rem;
}
</style>
