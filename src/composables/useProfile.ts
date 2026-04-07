import { ref, computed } from 'vue'
import { useUserStore } from '../stores/user'
import { updateProfileApi, sendEmailCodeApi, bindEmailApi } from '../api/auth'
import { uploadImage } from '../api/brand'
import { getBrand } from '../brand'
import { getAppCredentials } from '../utils/config'

export function useProfile() {
  const userStore = useUserStore()

  const showProfileModal = ref(false)
  const profileNickname = ref('')
  const profileAcctno = ref('')
  const profileEmail = ref('')
  const profileEmailCode = ref('')
  const emailEditing = ref(false)
  const emailSending = ref(false)
  const emailCooldown = ref(0)
  let emailCooldownTimer: ReturnType<typeof setInterval> | null = null
  const emailBtnText = computed(() => emailCooldown.value > 0 ? `${emailCooldown.value}s` : '发送验证码')
  const profileAvatarPreview = ref('')
  const profileAvatarFile = ref<File | null>(null)
  const profileLoading = ref(false)
  const profileErrMsg = ref('')
  const profileSuccessMsg = ref('')
  const avatarInputRef = ref<HTMLInputElement | null>(null)

  function startEmailCooldown() {
    emailCooldown.value = 60
    emailCooldownTimer = setInterval(() => {
      emailCooldown.value--
      if (emailCooldown.value <= 0 && emailCooldownTimer) {
        clearInterval(emailCooldownTimer)
        emailCooldownTimer = null
      }
    }, 1000)
  }

  async function handleSendEmailCode() {
    profileErrMsg.value = ''
    if (!profileEmail.value || !profileEmail.value.includes('@')) {
      profileErrMsg.value = '请输入正确的邮箱地址'
      return
    }
    emailSending.value = true
    try {
      const { appId } = await getAppCredentials()
      await sendEmailCodeApi({ app_id: appId, email: profileEmail.value.trim(), scene: 'bind_email' })
      profileSuccessMsg.value = '验证码已发送到邮箱'
      startEmailCooldown()
    } catch (err: unknown) {
      profileErrMsg.value = err instanceof Error ? err.message : '发送失败'
    } finally {
      emailSending.value = false
    }
  }

  function openProfileModal() {
    profileNickname.value = userStore.userInfo?.username || ''
    profileAcctno.value = userStore.userInfo?.acctno || ''
    profileEmail.value = userStore.userInfo?.email || ''
    profileEmailCode.value = ''
    emailEditing.value = !userStore.userInfo?.email
    profileAvatarPreview.value = userStore.avatarUrl
    profileAvatarFile.value = null
    profileErrMsg.value = ''
    profileSuccessMsg.value = ''
    showProfileModal.value = true
  }

  function closeProfileModal() {
    showProfileModal.value = false
    if (profileAvatarPreview.value?.startsWith('blob:')) {
      URL.revokeObjectURL(profileAvatarPreview.value)
    }
    if (emailCooldownTimer) { clearInterval(emailCooldownTimer); emailCooldownTimer = null }
  }

  function triggerAvatarSelect() {
    avatarInputRef.value?.click()
  }

  function onAvatarSelected(e: Event) {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    input.value = ''
    const validTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp']
    if (!validTypes.includes(file.type)) {
      profileErrMsg.value = '仅支持 jpg/png/gif/webp 格式'
      return
    }
    if (file.size > 2 * 1024 * 1024) {
      profileErrMsg.value = '头像大小不能超过 2MB'
      return
    }
    if (profileAvatarPreview.value?.startsWith('blob:')) {
      URL.revokeObjectURL(profileAvatarPreview.value)
    }
    profileAvatarFile.value = file
    profileAvatarPreview.value = URL.createObjectURL(file)
    profileErrMsg.value = ''
  }

  async function submitProfile() {
    profileErrMsg.value = ''
    profileSuccessMsg.value = ''
    if (profileAcctno.value && (profileAcctno.value.length < 2 || profileAcctno.value.length > 32)) {
      profileErrMsg.value = '账号长度需 2-32 位'
      return
    }
    const emailChanged = emailEditing.value && profileEmail.value.trim() !== (userStore.userInfo?.email || '')
    if (emailChanged) {
      if (!profileEmail.value.includes('@')) { profileErrMsg.value = '请输入正确的邮箱地址'; return }
      if (!profileEmailCode.value.trim()) { profileErrMsg.value = '请输入邮箱验证码'; return }
    }
    profileLoading.value = true
    try {
      let newAvatarUrl = ''
      let changed = false
      if (profileAvatarFile.value) {
        const uploadRes = await uploadImage(userStore.token, profileAvatarFile.value, 'avatar', {
          brandId: getBrand().id,
          phone: userStore.userInfo?.phone || '',
        })
        newAvatarUrl = uploadRes.url || ''
        profileAvatarFile.value = null
        changed = true
      }
      if (emailChanged) {
        const emailRes = await bindEmailApi({
          token: userStore.token,
          email: profileEmail.value.trim(),
          code: profileEmailCode.value.trim(),
        })
        userStore.updateUserInfo({ email: (emailRes as any).email || profileEmail.value.trim() })
        emailEditing.value = false
        profileEmailCode.value = ''
        changed = true
      }
      const changedFields: Record<string, string> = {}
      if (profileNickname.value !== (userStore.userInfo?.username || '')) changedFields.nickname = profileNickname.value
      if (profileAcctno.value !== (userStore.userInfo?.acctno || '')) changedFields.acctno = profileAcctno.value
      if (newAvatarUrl) changedFields.avatars = newAvatarUrl
      if (Object.keys(changedFields).length > 0) {
        const res = await updateProfileApi({ token: userStore.token, ...changedFields })
        userStore.updateUserInfo({
          username: (res as any).nickname || userStore.userInfo?.username,
          acctno: (res as any).acctno || userStore.userInfo?.acctno,
          avatars: (res as any).avatars || newAvatarUrl || userStore.userInfo?.avatars,
        })
        changed = true
      }
      profileSuccessMsg.value = changed ? '保存成功' : '没有需要修改的内容'
      if (changed) setTimeout(() => closeProfileModal(), 800)
    } catch (err: unknown) {
      profileErrMsg.value = err instanceof Error ? err.message : '保存失败'
    } finally {
      profileLoading.value = false
    }
  }

  return {
    showProfileModal, profileNickname, profileAcctno, profileEmail, profileEmailCode,
    emailEditing, emailSending, emailCooldown, emailBtnText,
    profileAvatarPreview, profileAvatarFile, profileLoading, profileErrMsg, profileSuccessMsg,
    avatarInputRef,
    openProfileModal, closeProfileModal, triggerAvatarSelect, onAvatarSelected,
    handleSendEmailCode, submitProfile,
  }
}
