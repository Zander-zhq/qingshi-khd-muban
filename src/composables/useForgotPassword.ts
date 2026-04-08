import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { sendEmailCodeApi, resetPasswordApi } from '../api/auth'
import { getAppCredentials } from '../utils/config'

export function useForgotPassword() {
  const router = useRouter()
  const appId = ref('')

  onMounted(async () => {
    const creds = await getAppCredentials()
    appId.value = creds.appId
  })

  const step = ref<'email' | 'reset'>('email')
  const email = ref('')
  const code = ref('')
  const newPassword = ref('')
  const confirmPassword = ref('')
  const loading = ref(false)
  const sendingCode = ref(false)
  const codeCooldown = ref(0)
  const errMsg = ref('')
  const successMsg = ref('')

  let cooldownTimer: ReturnType<typeof setInterval> | null = null

  const codeBtnText = computed(() =>
    codeCooldown.value > 0 ? `${codeCooldown.value}s` : '发送验证码'
  )

  function startCooldown() {
    codeCooldown.value = 60
    cooldownTimer = setInterval(() => {
      codeCooldown.value--
      if (codeCooldown.value <= 0 && cooldownTimer) {
        clearInterval(cooldownTimer)
        cooldownTimer = null
      }
    }, 1000)
  }

  onUnmounted(() => {
    if (cooldownTimer) clearInterval(cooldownTimer)
  })

  function clearMsg() {
    errMsg.value = ''
    successMsg.value = ''
  }

  async function handleSendCode() {
    clearMsg()
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    if (!emailRegex.test(email.value.trim())) {
      errMsg.value = '请输入正确的邮箱地址'
      return
    }

    sendingCode.value = true
    try {
      await sendEmailCodeApi({ app_id: appId.value, email: email.value.trim(), scene: 'reset_password' })
      successMsg.value = '验证码已发送到邮箱'
      startCooldown()
    } catch (err: unknown) {
      errMsg.value = err instanceof Error ? err.message : '发送失败'
    } finally {
      sendingCode.value = false
    }
  }

  function handleNextStep() {
    clearMsg()
    if (!email.value.trim()) { errMsg.value = '请输入邮箱地址'; return }
    if (!code.value.trim()) { errMsg.value = '请输入验证码'; return }
    step.value = 'reset'
  }

  async function handleResetPassword() {
    clearMsg()
    if (!newPassword.value || newPassword.value.length < 6) { errMsg.value = '密码至少6位'; return }
    if (newPassword.value !== confirmPassword.value) { errMsg.value = '两次密码不一致'; return }

    loading.value = true
    try {
      await resetPasswordApi({
        app_id: appId.value,
        email: email.value.trim(),
        code: code.value.trim(),
        new_password: newPassword.value,
      })
      successMsg.value = '密码重置成功，正在跳转...'
      setTimeout(() => router.push('/login'), 1500)
    } catch (err: unknown) {
      errMsg.value = err instanceof Error ? err.message : '重置失败'
    } finally {
      loading.value = false
    }
  }

  return {
    router,
    step,
    email,
    code,
    newPassword,
    confirmPassword,
    loading,
    sendingCode,
    codeCooldown,
    errMsg,
    successMsg,
    codeBtnText,
    clearMsg,
    handleSendCode,
    handleNextStep,
    handleResetPassword,
  }
}
