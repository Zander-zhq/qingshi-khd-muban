export interface BrandConfig {
  id: string
  brand_name: string
  product_name: string
  template: 'green' | 'orange' | 'dark'
  logo: string
  window_title: string
  about: string
  login_size: { width: number; height: number }
  main_size: { width: number; height: number; minWidth: number; minHeight: number }
}

const DEFAULT_BRAND: BrandConfig = {
  id: 'huayun',
  brand_name: '华云',
  product_name: '视频下载',
  template: 'orange',
  logo: 'app-icon.png',
  window_title: '华云·视频下载',
  about: '© 2024-2026 华云',
  login_size: { width: 500, height: 680 },
  main_size: { width: 1360, height: 860, minWidth: 1100, minHeight: 700 },
}

let currentBrand: BrandConfig = { ...DEFAULT_BRAND }
let loaded = false

export async function loadBrandConfig(): Promise<BrandConfig> {
  if (loaded) return currentBrand
  try {
    const saved = localStorage.getItem('brand_config')
    if (saved) {
      currentBrand = { ...DEFAULT_BRAND, ...JSON.parse(saved) }
    }
  } catch { /* use default */ }
  loaded = true
  return currentBrand
}

export function getBrand(): BrandConfig {
  if (!loaded) {
    const saved = localStorage.getItem('brand_config')
    if (saved) {
      try { currentBrand = { ...DEFAULT_BRAND, ...JSON.parse(saved) } } catch { /* */ }
    }
    loaded = true
  }
  return currentBrand
}

export function setBrand(config: BrandConfig) {
  currentBrand = config
  localStorage.setItem('brand_config', JSON.stringify(config))
}

export function getTemplate(): string {
  return getBrand().template
}

export const VERSION = 'V1.1.1'
