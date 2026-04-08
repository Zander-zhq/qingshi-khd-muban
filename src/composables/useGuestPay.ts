import { ref, computed, onUnmounted } from 'vue'
import { getBrand } from '../brand'
import { getAppCredentials } from '../utils/config'
import { fetchGuestPackages, createGuestOrder, queryGuestOrder } from '../api/pay'
import type { PayPackage } from '../api/pay'
import { logger } from '../utils/logger'
import QRCode from 'qrcode'

export function useGuestPay() {
  const brand = getBrand()

  const hasOnlinePay = computed(() => brand.pay_channel !== 'none' && (brand.pay_methods || []).length > 0)
  const hasWechat = computed(() => (brand.pay_methods || []).includes('wechat'))
  const hasAlipay = computed(() => (brand.pay_methods || []).includes('alipay'))

  const packages = ref<PayPackage[]>([])
  const packagesLoading = ref(false)
  const selectedPkg = ref<PayPackage | null>(null)
  const selectedPayMethod = ref<'wechat' | 'alipay'>('wechat')

  const payQrUrl = ref('')
  const payOrderNo = ref('')
  const showQrModal = ref(false)
  const payStatus = ref<'pending' | 'paid' | 'failed'>('pending')
  const payLoading = ref(false)
  const payErrMsg = ref('')
  const paySuccessMsg = ref('')

  let pollTimer: ReturnType<typeof setInterval> | null = null

  async function loadPackages() {
    packagesLoading.value = true
    try {
      const { appId } = await getAppCredentials()
      logger.log('guest-pay', '请求套餐列表', { appId })
      const res = await fetchGuestPackages(appId)
      logger.log('guest-pay', '套餐列表响应', res)
      packages.value = res.items || []
    } catch (err) {
      logger.error('guest-pay', '套餐列表请求失败', err)
      packages.value = []
    } finally {
      packagesLoading.value = false
    }
  }

  function formatPrice(price: number) {
    return (price / 100).toFixed(2)
  }

  function formatDuration(seconds?: number) {
    if (!seconds) return ''
    const days = Math.floor(seconds / 86400)
    if (days >= 365 && days % 365 === 0) return `${days / 365}年`
    if (days >= 30 && days % 30 === 0) return `${days / 30}个月`
    return `${days}天`
  }

  function selectPackage(pkg: PayPackage) {
    selectedPkg.value = pkg
    if (hasWechat.value) selectedPayMethod.value = 'wechat'
    else if (hasAlipay.value) selectedPayMethod.value = 'alipay'
  }

  async function startPay(acctno: string) {
    if (!selectedPkg.value || !acctno.trim()) return
    payErrMsg.value = ''
    payLoading.value = true
    try {
      const { appId } = await getAppCredentials()
      const payMethod = brand.pay_channel === 'hupijiao'
        ? `hupijiao_${selectedPayMethod.value}`
        : selectedPayMethod.value
      const res = await createGuestOrder(appId, {
        acctno: acctno.trim(),
        card_group_id: selectedPkg.value.id,
        payment_method: payMethod,
        brand_id: brand.id,
      })
      logger.log('guest-pay', '创建订单响应', res)
      payOrderNo.value = res.order_no

      const rawUrl = res.pay_url || res.code_url || res.qr_code || ''
      if (!rawUrl) {
        payErrMsg.value = '服务端未返回支付链接'
        return
      }
      payQrUrl.value = await QRCode.toDataURL(rawUrl, { width: 280, margin: 2 })
      payStatus.value = 'pending'
      showQrModal.value = true
      startPayPolling()
    } catch (err: unknown) {
      payErrMsg.value = err instanceof Error ? err.message : '创建订单失败'
    } finally {
      payLoading.value = false
    }
  }

  function startPayPolling() {
    stopPayPolling()
    pollTimer = setInterval(async () => {
      try {
        const res = await queryGuestOrder(payOrderNo.value)
        logger.log('guest-pay', '订单状态', { status: res.status, order_no: res.order_no })
        if (res.status === 'paid') {
          payStatus.value = 'paid'
          stopPayPolling()
          let msg = '支付成功！'
          if ((res as any).card_group_name) msg += ` (${(res as any).card_group_name})`
          if ((res as any).vip_expire_at) {
            const d = new Date((res as any).vip_expire_at)
            msg += `  到期：${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
          }
          paySuccessMsg.value = msg
        } else if (res.status === 'failed') {
          payStatus.value = 'failed'
          stopPayPolling()
        }
      } catch { /* polling error, continue */ }
    }, 3000)
  }

  function stopPayPolling() {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  function closeQrModal() {
    showQrModal.value = false
    stopPayPolling()
  }

  function resetPayState() {
    payErrMsg.value = ''
    paySuccessMsg.value = ''
    selectedPkg.value = null
    payOrderNo.value = ''
    payQrUrl.value = ''
    payStatus.value = 'pending'
    showQrModal.value = false
    stopPayPolling()
  }

  onUnmounted(() => stopPayPolling())

  return {
    hasOnlinePay,
    hasWechat,
    hasAlipay,
    packages,
    packagesLoading,
    selectedPkg,
    selectedPayMethod,
    payQrUrl,
    payOrderNo,
    showQrModal,
    payStatus,
    payLoading,
    payErrMsg,
    paySuccessMsg,
    loadPackages,
    formatPrice,
    formatDuration,
    selectPackage,
    startPay,
    closeQrModal,
    resetPayState,
  }
}
