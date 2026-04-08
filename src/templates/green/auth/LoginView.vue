<script setup lang="ts">
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import Button from 'primevue/button'
import Checkbox from 'primevue/checkbox'
import TitleBar from '../TitleBar.vue'
import { useLogin, getAccountAvatarSrc, maskPhone } from '../../../composables/useLogin'

const {
  router, cachedAvatar, acctno, password, rememberPwd, autoLogin, loading, errMsg,
  autoLoginCountdown, savedAccounts,
  dropdownAccounts, shouldShowDropdown, buttonLabel,
  selectAccount, handleDeleteAccount, onAcctInput, onAcctFocus, onAcctBlur,
  onAcctAreaLeave, cancelAcctAreaLeave, toggleDropdown,
  goRegister, goForgotPassword, handleLogin, handleButtonClick, onPasswordInput,
} = useLogin()
</script>

<template>
  <div class="window-shell">
    <div class="window-content">
      <TitleBar variant="auth" />
      <div class="banner">
        <div class="bc bc-1"></div>
        <div class="bc bc-2"></div>
        <div class="bc bc-3"></div>
        <div class="banner-title">登录</div>
      </div>

      <div class="body">
        <div class="avatar-ring">
          <div class="avatar">
            <img v-if="cachedAvatar" :src="cachedAvatar" class="avatar-img" alt="头像" />
            <i v-else class="pi pi-user"></i>
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

        <div class="bottom-links">
          <a class="bottom-link" href="#" @click.prevent="goRegister">注册账号</a>
          <span class="bottom-sep">|</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/recharge')">卡密充值</a>
          <span class="bottom-sep">|</span>
          <a class="bottom-link" href="#" @click.prevent="router.push('/unbind-device')">解绑设备</a>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.window-shell {
  height: 100vh;
  width: 100vw;
  background: #fff;
  animation: page-fade-in 0.35s ease-out;
}

.window-content {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: #fff;
  overflow-y: auto;
  position: relative;
}

.banner {
  height: 140px;
  position: relative;
  background: var(--qs-bg-gradient);
  flex-shrink: 0;
  overflow: visible;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bc {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.07);
  pointer-events: none;
}
.bc-1 { width: 200px; height: 200px; top: -100px; right: -40px; }
.bc-2 { width: 120px; height: 120px; bottom: -60px; left: 20px; }
.bc-3 { width: 70px; height: 70px; top: 10px; left: 38%; background: rgba(255,255,255,0.05); }

.banner-title {
  position: relative;
  z-index: 1;
  font-size: 1.8rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.15em;
  text-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0 40px 0;
  min-height: 0;
}

.avatar-ring {
  margin-top: -44px;
  margin-bottom: 14px;
  z-index: 2;
  padding: 4px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 4px 20px rgba(13, 148, 136, 0.12);
  flex-shrink: 0;
}

.avatar {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: linear-gradient(135deg, #ccfbf1, #f0fdfa);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--qs-primary);
  font-size: 2rem;
  overflow: hidden;
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
  gap: 14px;
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
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.15s;
  font-size: 0.75rem;
}

.acct-dropdown-toggle:hover {
  background: #f1f5f9;
  color: #64748b;
}

.acct-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.1);
  z-index: 100;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px;
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
  background: #f0fdfa;
}

.acct-item-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: linear-gradient(135deg, #2dd4bf, #14b8a6);
  color: #fff;
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
  font-size: 0.88rem;
  font-weight: 500;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.acct-item-phone {
  font-size: 0.75rem;
  color: #94a3b8;
}

.acct-item-del {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: #cbd5e1;
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
  background: #fef2f2;
  color: #ef4444;
}

.acct-empty {
  padding: 12px;
  text-align: center;
  color: #94a3b8;
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
  height: 46px;
  font-size: 0.95rem;
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

.err-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  color: #e74c3c;
  padding: 8px 12px;
  background: #fef2f2;
  border-radius: 8px;
  border: 1px solid #fecaca;
  margin: -4px 0;
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
  gap: 16px;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.check-item label {
  font-size: 0.85rem;
  color: var(--qs-text-secondary);
  cursor: pointer;
}

.link-text {
  font-size: 0.85rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  transition: color 0.15s;
}

.link-text:hover {
  color: var(--qs-primary-dark);
}

.submit-btn {
  width: 100%;
  height: 46px;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 23px;
  margin-top: 4px;
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
  padding: 12px 0 14px;
  text-align: center;
  flex-shrink: 0;
}

.bottom-link {
  display: inline-block;
  padding: 6px 10px;
  font-size: 0.82rem;
  color: var(--qs-primary);
  text-decoration: none;
  font-weight: 500;
  cursor: pointer;
  transition: color 0.15s;
}

.bottom-link:hover {
  color: var(--qs-primary-dark);
}

.bottom-sep {
  color: #cbd5e1;
  font-size: 0.8rem;
}
</style>
