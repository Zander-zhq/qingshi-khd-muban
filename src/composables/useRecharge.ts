import { ref } from 'vue'
import { useUserStore } from '../stores/user'
import { redeemCardInnerApi } from '../api/auth'
import { startHeartbeat } from '../utils/heartbeat'

export function useRecharge(forceExpired: ReturnType<typeof ref<boolean>>, showExpired: ReturnType<typeof ref<boolean>>) {
  const userStore = useUserStore()

  const showRechargeModal = ref(false)
  const rechargeCardKey = ref('')
  const rechargeLoading = ref(false)
  const rechargeErrMsg = ref('')
  const rechargeSuccessMsg = ref('')

  function openRechargeModal() {
    rechargeCardKey.value = ''
    rechargeErrMsg.value = ''
    rechargeSuccessMsg.value = ''
    showRechargeModal.value = true
  }

  function closeRechargeModal() {
    showRechargeModal.value = false
    if (forceExpired.value) {
      showExpired.value = true
    }
  }

  async function submitRecharge() {
    rechargeErrMsg.value = ''
    rechargeSuccessMsg.value = ''
    if (!rechargeCardKey.value.trim()) { rechargeErrMsg.value = '请输入卡密'; return }
    rechargeLoading.value = true
    try {
      const res = await redeemCardInnerApi({
        token: userStore.token,
        card_key: rechargeCardKey.value.trim(),
      })
      const cardType = (res as any).card_type || ''
      const expireAt = (res as any).vip_expire_at || ''
      let msg = '充值成功！'
      if (cardType) msg += ` (${cardType})`
      if (expireAt) {
        const d = new Date(expireAt)
        msg += `\n到期时间：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
      }
      rechargeSuccessMsg.value = msg
      rechargeCardKey.value = ''
      if (forceExpired.value) {
        forceExpired.value = false
        showExpired.value = false
        if (userStore.token) startHeartbeat(userStore.token)
      }
    } catch (err: unknown) {
      rechargeErrMsg.value = err instanceof Error ? err.message : '充值失败'
    } finally {
      rechargeLoading.value = false
    }
  }

  return {
    showRechargeModal, rechargeCardKey, rechargeLoading, rechargeErrMsg, rechargeSuccessMsg,
    openRechargeModal, closeRechargeModal, submitRecharge,
  }
}
