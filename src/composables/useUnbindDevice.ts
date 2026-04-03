import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useUserStore } from '../stores/user'
import { unbindDeviceInnerApi } from '../api/auth'
import { stopHeartbeat } from '../utils/heartbeat'
import { switchToLoginLayout } from '../utils/window'
import { getDeviceId } from '../utils/device'

export function useUnbindDevice() {
  const router = useRouter()
  const userStore = useUserStore()

  const showUnbindModal = ref(false)
  const unbindLoading = ref(false)
  const unbindErrMsg = ref('')
  const unbindSuccessMsg = ref('')

  function openUnbindModal() {
    unbindErrMsg.value = ''
    unbindSuccessMsg.value = ''
    showUnbindModal.value = true
  }

  async function submitUnbind() {
    unbindErrMsg.value = ''
    unbindSuccessMsg.value = ''
    unbindLoading.value = true
    try {
      const deviceId = await getDeviceId()
      await unbindDeviceInnerApi({ token: userStore.token, device_id: deviceId })
      unbindSuccessMsg.value = '解绑成功，即将退出登录...'
      setTimeout(async () => {
        showUnbindModal.value = false
        stopHeartbeat()
        userStore.logout()
        await switchToLoginLayout(router)
      }, 1500)
    } catch (err: unknown) {
      unbindErrMsg.value = err instanceof Error ? err.message : '解绑失败'
    } finally {
      unbindLoading.value = false
    }
  }

  return {
    showUnbindModal, unbindLoading, unbindErrMsg, unbindSuccessMsg,
    openUnbindModal, submitUnbind,
  }
}
