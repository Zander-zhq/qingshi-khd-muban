import { invoke } from '@tauri-apps/api/core'

export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL as string

let cachedCredentials: { appId: string; appKey: string } | null = null

export async function getAppCredentials(): Promise<{ appId: string; appKey: string }> {
  if (cachedCredentials) return cachedCredentials
  const [appId, appKey] = await invoke<[string, string]>('get_app_credentials')
  cachedCredentials = { appId, appKey }
  return cachedCredentials
}
