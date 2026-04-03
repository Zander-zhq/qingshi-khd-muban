import { ref } from 'vue'
import { useUserStore } from '../stores/user'
import { changePasswordApi } from '../api/auth'

export function useChangePassword() {
  const userStore = useUserStore()

  const showChangePwdModal = ref(false)
  const cpOldPwd = ref('')
  const cpNewPwd = ref('')
  const cpConfirmPwd = ref('')
  const cpLoading = ref(false)
  const cpErrMsg = ref('')
  const cpSuccessMsg = ref('')

  function openChangePwdModal() {
    cpOldPwd.value = ''
    cpNewPwd.value = ''
    cpConfirmPwd.value = ''
    cpErrMsg.value = ''
    cpSuccessMsg.value = ''
    showChangePwdModal.value = true
  }

  async function submitChangePassword() {
    cpErrMsg.value = ''
    cpSuccessMsg.value = ''
    if (!cpOldPwd.value) { cpErrMsg.value = '请输入原密码'; return }
    if (!cpNewPwd.value || cpNewPwd.value.length < 6) { cpErrMsg.value = '新密码至少6位'; return }
    if (cpNewPwd.value !== cpConfirmPwd.value) { cpErrMsg.value = '两次密码不一致'; return }
    cpLoading.value = true
    try {
      await changePasswordApi({
        token: userStore.token,
        old_password: cpOldPwd.value,
        new_password: cpNewPwd.value,
      })
      cpSuccessMsg.value = '密码修改成功'
      setTimeout(() => { showChangePwdModal.value = false }, 800)
    } catch (err: unknown) {
      cpErrMsg.value = err instanceof Error ? err.message : '修改失败'
    } finally {
      cpLoading.value = false
    }
  }

  return {
    showChangePwdModal, cpOldPwd, cpNewPwd, cpConfirmPwd, cpLoading, cpErrMsg, cpSuccessMsg,
    openChangePwdModal, submitChangePassword,
  }
}
