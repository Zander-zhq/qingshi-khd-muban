import { invoke } from '@tauri-apps/api/core'
import type { ServerBrand } from './api/brand'
import { templateLabelToId } from './api/brand'
import { appStorage } from './utils/storage'
import { logger } from './utils/logger'

/** 奖励类型：会员时长（秒） / 积分数量；与服务端保持一致 */
export type BrandRewardType = 'membership' | 'points'

export interface BrandConfig {
  id: string
  brand_name: string
  product_name: string
  template: 'green' | 'orange' | 'dark'
  logo: string
  about: string
  website: string
  tutorial_url: string
  contact_images: string[]
  data_version: number
  reg_reward_type: BrandRewardType
  reg_reward_value: number
  invite_invitee_reward_type: BrandRewardType
  invite_invitee_reward_value: number
  invite_inviter_reward_type: BrandRewardType
  invite_inviter_reward_value: number
  checkin_reward_type: BrandRewardType
  checkin_reward_value: number
  pay_channel: 'none' | 'enterprise' | 'hupijiao'
  pay_methods: string[]
}

const DEFAULT_BRAND: BrandConfig = {
  id: 'huayun',
  brand_name: '华云',
  product_name: '视频下载',
  template: 'green',
  logo: 'app-icon.png',
  about: '© 2024-2026 华云',
  website: '',
  tutorial_url: '',
  contact_images: [],
  data_version: 0,
  reg_reward_type: 'membership',
  reg_reward_value: 0,
  invite_invitee_reward_type: 'membership',
  invite_invitee_reward_value: 0,
  invite_inviter_reward_type: 'membership',
  invite_inviter_reward_value: 0,
  checkin_reward_type: 'membership',
  checkin_reward_value: 0,
  pay_channel: 'none',
  pay_methods: [],
}

const VALID_TEMPLATES = ['green', 'orange', 'dark'] as const

let currentBrand: BrandConfig = { ...DEFAULT_BRAND }
let loaded = false

export function coerceBrandRewardType(raw: unknown): BrandRewardType {
  if (raw === 'points' || raw === 'point' || raw === 2 || raw === '2' || raw === '积分') return 'points'
  return 'membership'
}

export function coerceBrandRewardValue(raw: unknown): number {
  const n = Number(raw)
  return Number.isFinite(n) && n >= 0 ? Math.floor(n) : 0
}

/* ─── 服务端 → 客户端 格式转换 ─── */

export function serverBrandToConfig(s: ServerBrand): BrandConfig {
  const tpl = templateLabelToId(s.template)
  return {
    id: s.brand_id,
    brand_name: s.name,
    product_name: s.product_name,
    template: VALID_TEMPLATES.includes(tpl as any) ? (tpl as BrandConfig['template']) : 'green',
    logo: s.logo || '',
    about: s.about_info || '',
    website: s.website_url || '',
    tutorial_url: s.tutorial_url || '',
    contact_images: Array.isArray(s.contact_images) ? s.contact_images : [],
    data_version: s.data_version ?? 0,
    reg_reward_type: coerceBrandRewardType(s.reg_reward_type),
    reg_reward_value: coerceBrandRewardValue(s.reg_reward_value),
    invite_invitee_reward_type: coerceBrandRewardType(s.invite_invitee_reward_type),
    invite_invitee_reward_value: coerceBrandRewardValue(s.invite_invitee_reward_value),
    invite_inviter_reward_type: coerceBrandRewardType(s.invite_inviter_reward_type),
    invite_inviter_reward_value: coerceBrandRewardValue(s.invite_inviter_reward_value),
    checkin_reward_type: coerceBrandRewardType(s.checkin_reward_type),
    checkin_reward_value: coerceBrandRewardValue(s.checkin_reward_value),
    pay_channel: (['none', 'enterprise', 'hupijiao'].includes(s.pay_channel || '') ? s.pay_channel : 'none') as BrandConfig['pay_channel'],
    pay_methods: Array.isArray(s.pay_methods) ? s.pay_methods : [],
  }
}

/* ─── localStorage 读写 ─── */

function parseSaved(): BrandConfig | null {
  try {
    const saved = appStorage.getItem('brand_config')
    if (!saved) return null
    const parsed = { ...DEFAULT_BRAND, ...JSON.parse(saved) }
    if (!VALID_TEMPLATES.includes(parsed.template)) {
      parsed.template = DEFAULT_BRAND.template
    }
    if (!Array.isArray(parsed.contact_images)) {
      parsed.contact_images = []
    }
    parsed.reg_reward_type = coerceBrandRewardType(parsed.reg_reward_type)
    parsed.reg_reward_value = coerceBrandRewardValue(parsed.reg_reward_value)
    parsed.invite_invitee_reward_type = coerceBrandRewardType(parsed.invite_invitee_reward_type)
    parsed.invite_invitee_reward_value = coerceBrandRewardValue(parsed.invite_invitee_reward_value)
    parsed.invite_inviter_reward_type = coerceBrandRewardType(parsed.invite_inviter_reward_type)
    parsed.invite_inviter_reward_value = coerceBrandRewardValue(parsed.invite_inviter_reward_value)
    parsed.checkin_reward_type = coerceBrandRewardType(parsed.checkin_reward_type)
    parsed.checkin_reward_value = coerceBrandRewardValue(parsed.checkin_reward_value)
    return parsed
  } catch {
    return null
  }
}

export function getBrand(): BrandConfig {
  if (!loaded) {
    currentBrand = parseSaved() || { ...DEFAULT_BRAND }
    loaded = true
  }
  return currentBrand
}

export function setBrand(config: BrandConfig) {
  currentBrand = config
  loaded = true
  appStorage.setItem('brand_config', JSON.stringify(config))
}

/** 只保存 active brand_id，用于下次启动时选择品牌 */
export function setActiveBrandId(brandId: string) {
  appStorage.setItem('active_brand_id', brandId)
}

export function getActiveBrandId(): string {
  return appStorage.getItem('active_brand_id') || getBrand().id
}

/* ─── 图片 URL 辅助 ─── */

const API_BASE = import.meta.env.VITE_API_BASE_URL as string

export function resolveImageUrl(path: string): string {
  if (!path) return ''
  if (path.startsWith('data:') || path.startsWith('http') || path.startsWith('blob:')) return path
  if (path.startsWith('/uploads') || path.startsWith('/static')) return API_BASE + path
  return '/' + path
}

let logoCacheData = ''

export function getBrandLogo(): string {
  if (logoCacheData) return logoCacheData
  const cached = appStorage.getItem('brand_logo_data')
  const cachedUrl = appStorage.getItem('brand_logo_url')
  const logo = getBrand().logo
  if (cached && cachedUrl === logo) {
    logoCacheData = cached
    return cached
  }
  if (!logo) return '/app-icon.png'
  return resolveImageUrl(logo) || '/app-icon.png'
}

function resizeImageToBase64(imgSrc: string, maxSize = 128): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      const w = Math.min(img.naturalWidth, maxSize)
      const h = Math.min(img.naturalHeight, maxSize)
      const size = Math.min(w, h)
      const canvas = document.createElement('canvas')
      canvas.width = size
      canvas.height = size
      const ctx = canvas.getContext('2d')!
      ctx.imageSmoothingQuality = 'high'
      ctx.drawImage(img, 0, 0, size, size)
      resolve(canvas.toDataURL('image/png'))
    }
    img.onerror = () => reject(new Error('Image load failed'))
    img.src = imgSrc
  })
}

async function downloadImageAsBase64(url: string): Promise<string> {
  try {
    const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
    const resp = await tauriFetch(url)
    if (resp.ok) {
      const blob = await resp.blob()
      const blobUrl = URL.createObjectURL(blob)
      try {
        return await resizeImageToBase64(blobUrl)
      } finally {
        URL.revokeObjectURL(blobUrl)
      }
    }
  } catch { /* tauri fetch failed, try canvas */ }

  try {
    return await resizeImageToBase64(url)
  } catch { /* canvas fallback also failed */ }

  return ''
}

async function cacheBrandLogo(): Promise<void> {
  const logo = getBrand().logo
  if (!logo) return
  const cachedUrl = appStorage.getItem('brand_logo_url')
  if (cachedUrl === logo && appStorage.getItem('brand_logo_data')) {
    logoCacheData = appStorage.getItem('brand_logo_data')!
    return
  }
  const fullUrl = resolveImageUrl(logo)
  const base64 = await downloadImageAsBase64(fullUrl)
  if (base64) {
    logoCacheData = base64
    try {
      appStorage.setItem('brand_logo_data', base64)
      appStorage.setItem('brand_logo_url', logo)
    } catch { /* quota exceeded */ }
  } else {
    logger.warn('brand', 'cacheBrandLogo 缓存失败', { url: fullUrl })
  }
}

export function formatReward(type: BrandRewardType, value: number): string {
  if (!value || value <= 0) return ''
  if (type === 'points') return `${value} 积分`
  const days = Math.floor(value / 86400)
  const hours = Math.floor((value % 86400) / 3600)
  if (days > 0 && hours > 0) return `${days}天${hours}小时会员`
  if (days > 0) return `${days}天会员`
  if (hours > 0) return `${hours}小时会员`
  return `${value}秒会员`
}

export function getWindowTitle(): string {
  const b = getBrand()
  return `${b.brand_name}·${b.product_name}`
}

export function getTemplate(): string {
  return getBrand().template
}

/* ─── 全局主题 CSS 变量 ─── */

const THEME_MAP: Record<string, Record<string, string>> = {
  green: {
    '--app-primary': '#22c55e',
    '--app-primary-hover': '#16a34a',
    '--app-primary-light': '#f0fdf4',
    '--app-primary-light-hover': '#dcfce7',
    '--app-primary-border': '#86efac',
    '--app-primary-text': '#ffffff',
  },
  orange: {
    '--app-primary': '#F97316',
    '--app-primary-hover': '#EA580C',
    '--app-primary-light': '#FFF7ED',
    '--app-primary-light-hover': '#FFEDD5',
    '--app-primary-border': '#FDBA74',
    '--app-primary-text': '#ffffff',
  },
  dark: {
    '--app-primary': '#22D3EE',
    '--app-primary-hover': '#0891B2',
    '--app-primary-light': '#083344',
    '--app-primary-light-hover': '#0e4a5c',
    '--app-primary-border': '#22D3EE',
    '--app-primary-text': '#0F172A',
  },
}

export function applyThemeVars(): void {
  const vars = THEME_MAP[getTemplate()] || THEME_MAP.green
  const root = document.documentElement
  for (const [key, value] of Object.entries(vars)) {
    root.style.setProperty(key, value)
  }
}

/* ─── 从服务器同步加密品牌配置 ─── */

let syncing = false

/**
 * 从本地加密文件或服务器同步品牌配置（无需 token）。
 * 1. 读取本地加密配置文件 → 获取 data_version
 * 2. 向服务器发送 data_version 对比
 * 3. 有更新则保存新密文 + 解密 + 应用到 localStorage
 * 返回 true 表示配置有变化。
 */
async function doSyncBrand(): Promise<boolean> {
  let localVersion = 0
  let localBrandId: string | undefined

  const localJson = await invoke<string | null>('read_brand_config')
  if (localJson) {
    try {
      const local = JSON.parse(localJson) as ServerBrand
      localVersion = local.data_version ?? 0
      localBrandId = local.brand_id
    } catch { /* 本地配置损坏，当作首次 */ }
  }

  const { syncBrandConfig } = await import('./api/brand')
  const res = await syncBrandConfig(localBrandId || getActiveBrandId(), localVersion)

  if (res.updated && res.config) {
    await invoke('save_brand_config', { encryptedBase64: res.config })
    const json = await invoke<string>('decrypt_brand_config', { encryptedBase64: res.config })
    const serverBrand = JSON.parse(json) as ServerBrand
    const config = serverBrandToConfig(serverBrand)
    setBrand(config)
    setActiveBrandId(config.id)
    return true
  }

  if (localJson && !res.updated) {
    const serverBrand = JSON.parse(localJson) as ServerBrand
    const config = serverBrandToConfig(serverBrand)
    const local = getBrand()
    if (config.id !== local.id || config.data_version !== local.data_version) {
      setBrand(config)
      setActiveBrandId(config.id)
      return true
    }
  }

  return false
}

/**
 * 应用启动时调用（Vue 初始化之前）。
 * 无需 token，先同步品牌配置到 localStorage，
 * 这样后续 router 初始化时 getBrand() 能拿到最新数据。
 */
async function syncTrayIcon(): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const b = getBrand()
    const logoData = getBrandLogo()
    const title = `${b.brand_name}·${b.product_name}`
    await invoke('update_tray', {
      tooltip: title,
      iconData: logoData.startsWith('data:') ? logoData : '',
    })
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().setTitle(title)
  } catch { /* tray/title update failed silently */ }
}

export async function initBrand(): Promise<void> {
  try {
    await doSyncBrand()
  } catch {
    /* 网络失败时静默降级，用本地缓存或默认值 */
  }
  await cacheBrandLogo().catch(() => {})
  syncTrayIcon().catch(() => {})
}

/**
 * 运行时手动触发同步（如品牌管理保存后）。
 * 如果模板变了会自动刷新页面。
 */
export async function syncBrandFromServer(): Promise<void> {
  if (syncing) return
  syncing = true
  try {
    const oldTemplate = getBrand().template
    const changed = await doSyncBrand()
    if (changed && getBrand().template !== oldTemplate) {
      window.location.reload()
    }
  } catch {
    /* 同步失败不影响应用运行 */
  } finally {
    syncing = false
  }
}

export const VERSION: string = import.meta.env.VITE_APP_VERSION || 'V1.1.4'
