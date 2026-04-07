import { ref, onMounted } from 'vue'
import { getBrand, resolveImageUrl } from '../brand'
import { appStorage } from '../utils/storage'
import { logger } from '../utils/logger'

const CACHE_KEY = 'contact_images_cache'
const CACHE_URLS_KEY = 'contact_images_urls'

export function useContact() {
  const brand = getBrand()
  const hasContactImages = brand.contact_images && brand.contact_images.length > 0
  const showContactFloat = ref(hasContactImages)
  const showContactModal = ref(false)
  const contactHighlight = ref(false)
  const cachedImages = ref<string[]>([])

  function closeContactFloat() {
    showContactFloat.value = false
  }

  function restoreContactFloat() {
    showContactFloat.value = true
    contactHighlight.value = true
    setTimeout(() => { contactHighlight.value = false }, 2000)
  }

  async function cacheContactImages() {
    if (!hasContactImages) return

    const urls = brand.contact_images.map(img => resolveImageUrl(img))
    const urlsKey = JSON.stringify(urls)
    const savedUrls = appStorage.getItem(CACHE_URLS_KEY)

    if (savedUrls === urlsKey) {
      const cached = appStorage.getItem(CACHE_KEY)
      if (cached) {
        try {
          cachedImages.value = JSON.parse(cached)
          return
        } catch { /* cache corrupted */ }
      }
    }

    const results: string[] = []
    for (const url of urls) {
      try {
        const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
        const resp = await tauriFetch(url)
        if (!resp.ok) { results.push(url); continue }
        const blob = await resp.blob()
        const reader = new FileReader()
        const base64 = await new Promise<string>((resolve, reject) => {
          reader.onloadend = () => reader.result ? resolve(reader.result as string) : reject(new Error('FileReader result is null'))
          reader.onerror = () => reject(reader.error)
          reader.readAsDataURL(blob)
        })
        results.push(base64)
      } catch (err) {
        logger.warn('contact', '联系图片缓存失败，使用URL', { url })
        results.push(url)
      }
    }

    cachedImages.value = results
    try {
      appStorage.setItem(CACHE_KEY, JSON.stringify(results))
      appStorage.setItem(CACHE_URLS_KEY, urlsKey)
    } catch { /* quota exceeded */ }
  }

  onMounted(() => {
    if (hasContactImages) cacheContactImages()
  })

  return {
    hasContactImages,
    showContactFloat,
    showContactModal,
    contactHighlight,
    cachedImages,
    closeContactFloat,
    restoreContactFloat,
  }
}
