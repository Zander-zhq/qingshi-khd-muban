import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useUserStore } from '../stores/user'
import { logger } from '../utils/logger'
import { switchToLoginLayout } from '../utils/window'
import { startHeartbeat, stopHeartbeat, callLogoutApi, setHeartbeatCallbacks } from '../utils/heartbeat'
import { showDialog } from '../utils/dialog'

export function useAuth() {
  const router = useRouter()
  const userStore = useUserStore()
  const displayName = computed(() => userStore.userInfo?.username || '用户')

  const showBanned = ref(false)
  const bannedTitle = ref('账号已被禁用')
  const bannedMsg = ref('')
  const bannedCountdown = ref(5)
  const showExpired = ref(false)
  const expiredMsg = ref('')
  const forceExpired = ref(false)

  function countdownLogout(seconds: number) {
    bannedCountdown.value = seconds
    const t = setInterval(async () => {
      bannedCountdown.value--
      if (bannedCountdown.value <= 0) {
        clearInterval(t)
        userStore.logout()
        await switchToLoginLayout(router)
      }
    }, 1000)
  }

  function initHeartbeat() {
    if (userStore.token) {
      startHeartbeat(userStore.token)
    }
    setHeartbeatCallbacks({
      onBanned(msg) {
        bannedTitle.value = '账号已被禁用'
        bannedMsg.value = msg
        showBanned.value = true
        countdownLogout(5)
      },
      onExpired(msg) {
        expiredMsg.value = msg
        showExpired.value = true
        forceExpired.value = true
      },
      async onSessionExpired(msg) {
        logger.warn('auth', '会话过期', { msg })
        await showDialog({ title: '登录已过期', message: msg || '您的登录已过期，请重新登录' })
        userStore.logout()
        await switchToLoginLayout(router)
      },
      async onDeviceKicked(msg) {
        bannedTitle.value = '设备已被下线'
        bannedMsg.value = msg
        showBanned.value = true
        countdownLogout(5)
      },
      async onDeviceMismatch(msg) {
        await showDialog({ title: '设备异常', message: msg })
        userStore.logout()
        await switchToLoginLayout(router)
      },
      async onServerUnreachable(msg) {
        logger.warn('auth', '服务器不可达，强制下线', { msg })
        await showDialog({ title: '连接断开', message: msg || '无法连接到服务器，请检查网络后重新登录' })
        userStore.logout()
        await switchToLoginLayout(router)
      },
    })
  }

  async function handleLogout() {
    logger.log('auth', '用户退出登录')
    stopHeartbeat()
    if (userStore.token) {
      await callLogoutApi(userStore.token)
    }
    userStore.logout()
    await switchToLoginLayout(router)
  }

  function destroyHeartbeat() {
    stopHeartbeat()
  }

  return {
    userStore, router, displayName,
    showBanned, bannedTitle, bannedMsg, bannedCountdown,
    showExpired, expiredMsg, forceExpired,
    initHeartbeat, destroyHeartbeat, handleLogout, countdownLogout,
  }
}
