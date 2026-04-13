import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { userRegisterApi } from '../api/auth'
import { getDeviceId, getInstanceId } from '../utils/device'
import { getAppCredentials } from '../utils/config'

export function useRegister() {
  const router = useRouter()

  const phone = ref('')
  const password = ref('')
  const confirmPassword = ref('')
  const inviteCode = ref('')
  const deviceId = ref('')
  const appId = ref('')
  const loading = ref(false)
  const errMsg = ref('')
  const successMsg = ref('')

  onMounted(async () => {
    const [did, creds] = await Promise.all([getDeviceId(), getAppCredentials()])
    deviceId.value = did
    appId.value = creds.appId
  })

  function clearMsg() {
    errMsg.value = ''
    successMsg.value = ''
  }

  async function handleRegister() {
    clearMsg()

    const phoneVal = phone.value.trim()
    if (!phoneVal) { errMsg.value = '请输入手机号'; return }
    if (!/^1[3-9]\d{9}$/.test(phoneVal)) { errMsg.value = '手机号格式不正确'; return }
    if (!password.value || password.value.length < 6) { errMsg.value = '密码至少6位'; return }
    if (password.value.length > 18) { errMsg.value = '密码最长18位'; return }
    if (password.value !== confirmPassword.value) { errMsg.value = '两次密码不一致'; return }

    loading.value = true
    try {
      const params: Record<string, string> = {
        app_id: appId.value,
        phone: phoneVal,
        password: password.value,
        device_id: deviceId.value,
        instance_id: getInstanceId(),
      }
      const trimmedInvite = inviteCode.value.trim()
      if (trimmedInvite) params.invite_code = trimmedInvite

      await userRegisterApi(params as any)
      successMsg.value = '注册成功，正在跳转...'
      setTimeout(() => router.push('/login'), 1500)
    } catch (err: unknown) {
      errMsg.value = err instanceof Error ? err.message : '注册失败'
    } finally {
      loading.value = false
    }
  }

  return {
    router,
    phone,
    password,
    confirmPassword,
    inviteCode,
    loading,
    errMsg,
    successMsg,
    clearMsg,
    handleRegister,
  }
}
