import { ref, computed } from 'vue'
import { appStorage } from '../utils/storage'
import { getBrand } from '../brand'
import { getAppCredentials } from '../utils/config'
import { userCheckinApi, type CheckinInfo, type CheckinResult } from '../api/auth'
import { useUserStore } from '../stores/user'

const CHECKIN_KEY = 'checkin_info'
const checkinInfo = ref<CheckinInfo | null>(null)
const checkinLoading = ref(false)

function loadFromStorage() {
  if (checkinInfo.value) return
  const saved = appStorage.getItem(CHECKIN_KEY)
  if (saved) {
    try { checkinInfo.value = JSON.parse(saved) } catch { /* ignore */ }
  }
}

export function useCheckin() {
  loadFromStorage()

  const canCheckin = computed(() => checkinInfo.value?.can_checkin ?? false)
  const rewardSummary = computed(() => checkinInfo.value?.reward_summary ?? '')
  const showCheckinHint = computed(() => canCheckin.value && !!rewardSummary.value)

  function setCheckinInfo(info: CheckinInfo | null) {
    checkinInfo.value = info
    if (info) {
      appStorage.setItem(CHECKIN_KEY, JSON.stringify(info))
    } else {
      appStorage.removeItem(CHECKIN_KEY)
    }
  }

  async function doCheckin(): Promise<CheckinResult | null> {
    if (checkinLoading.value) return null
    checkinLoading.value = true
    try {
      const { appId } = await getAppCredentials()
      const userStore = useUserStore()
      const brand = getBrand()
      const res = await userCheckinApi({
        app_id: appId,
        token: userStore.token,
        brand_id: brand.id,
      })
      if (res.checkin) {
        setCheckinInfo(res.checkin)
      }
      if (res.vip_expire_at) {
        userStore.updateUserInfo({ vip_expire_at: res.vip_expire_at })
      }
      if (res.fen != null) {
        userStore.updateUserInfo({ fen: res.fen })
      }
      return res
    } finally {
      checkinLoading.value = false
    }
  }

  function clearCheckin() {
    checkinInfo.value = null
    appStorage.removeItem(CHECKIN_KEY)
  }

  return {
    checkinInfo,
    checkinLoading,
    canCheckin,
    rewardSummary,
    showCheckinHint,
    setCheckinInfo,
    doCheckin,
    clearCheckin,
  }
}
