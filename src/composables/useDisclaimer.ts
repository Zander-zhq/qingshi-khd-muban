import { ref, computed } from 'vue'
import { getBrand } from '../brand'
import { appStorage } from '../utils/storage'
import { useUserStore } from '../stores/user'

function storageKey(userId: number | string): string {
  return `disclaimer_accepted_${userId}`
}

export function useDisclaimer() {
  const brand = getBrand()
  const userStore = useUserStore()
  const showDisclaimerModal = ref(false)

  const hasDisclaimer = computed(() => !!brand.disclaimer?.trim())

  function openDisclaimer() {
    if (hasDisclaimer.value) {
      showDisclaimerModal.value = true
    }
  }

  function closeDisclaimer() {
    showDisclaimerModal.value = false
  }

  function acceptDisclaimer() {
    const userId = userStore.userInfo?.id
    if (userId) {
      appStorage.setItem(storageKey(userId), '1')
    }
    showDisclaimerModal.value = false
  }

  /**
   * 首次登录检测：如果用户从未接受过免责声明，自动弹出。
   * 在 MainLayout 的 onMounted 中调用。
   */
  function checkAndShowOnFirstLogin() {
    if (!hasDisclaimer.value) return
    const userId = userStore.userInfo?.id
    if (!userId) return
    const accepted = appStorage.getItem(storageKey(userId))
    if (!accepted) {
      showDisclaimerModal.value = true
    }
  }

  return {
    showDisclaimerModal,
    hasDisclaimer,
    disclaimerText: brand.disclaimer || '',
    openDisclaimer,
    closeDisclaimer,
    acceptDisclaimer,
    checkAndShowOnFirstLogin,
  }
}
