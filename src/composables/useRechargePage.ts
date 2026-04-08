import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { redeemCardApi } from '../api/auth'
import { getAppCredentials } from '../utils/config'
import { useGuestPay } from './useGuestPay'

export function useRechargePage() {
  const router = useRouter()
  const appId = ref('')

  const guestPay = useGuestPay()

  const activeTab = ref<'online' | 'card'>('card')

  onMounted(async () => {
    const creds = await getAppCredentials()
    appId.value = creds.appId
    if (guestPay.hasOnlinePay.value) {
      activeTab.value = 'online'
      guestPay.loadPackages()
    }
  })

  const acctno = ref('')
  const cardKey = ref('')
  const loading = ref(false)
  const errMsg = ref('')
  const successMsg = ref('')

  function clearMsg() {
    errMsg.value = ''
    successMsg.value = ''
  }

  function switchTab(tab: 'online' | 'card') {
    activeTab.value = tab
    clearMsg()
    guestPay.resetPayState()
  }

  async function handleRecharge() {
    clearMsg()

    if (!acctno.value.trim()) { errMsg.value = '请输入账号（手机号）'; return }
    if (!cardKey.value.trim()) { errMsg.value = '请输入卡密'; return }

    loading.value = true
    try {
      const res = await redeemCardApi({
        app_id: appId.value,
        acctno: acctno.value.trim(),
        card_key: cardKey.value.trim(),
      })
      const cardType = (res as any).card_type || ''
      const expireAt = (res as any).vip_expire_at || ''
      let msg = '充值成功！'
      if (cardType) msg += ` (${cardType})`
      if (expireAt) {
        const d = new Date(expireAt)
        msg += `  到期：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
      }
      successMsg.value = msg
      cardKey.value = ''
      setTimeout(() => router.push('/login'), 1500)
    } catch (err: unknown) {
      errMsg.value = err instanceof Error ? err.message : '充值失败'
    } finally {
      loading.value = false
    }
  }

  function handleStartPay() {
    if (!acctno.value.trim()) {
      guestPay.payErrMsg.value = '请输入账号（手机号）'
      return
    }
    guestPay.startPay(acctno.value)
  }

  return {
    router,
    activeTab,
    acctno,
    cardKey,
    loading,
    errMsg,
    successMsg,
    clearMsg,
    switchTab,
    handleRecharge,
    handleStartPay,
    ...guestPay,
  }
}
