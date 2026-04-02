import { invoke } from '@tauri-apps/api/core'
import { post } from './request'

export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL as string

let cachedCredentials: { appId: string; appKey: string } | null = null

export async function getAppCredentials(): Promise<{ appId: string; appKey: string }> {
  if (cachedCredentials) return cachedCredentials
  const [appId, appKey] = await invoke<[string, string]>('get_app_credentials')
  cachedCredentials = { appId, appKey }
  return cachedCredentials
}

let cachedAppConfig: { unbindTip: string; notice: string | null } | null = null

export async function fetchAppConfig(): Promise<{ unbindTip: string; notice: string | null }> {
  if (cachedAppConfig) return cachedAppConfig
  try {
    const { appId, appKey } = await getAppCredentials()
    const res = await post<Record<string, unknown>>('/client/init', { app_id: appId, app_key: appKey })
    cachedAppConfig = {
      unbindTip: typeof (res as any).unbind_tip === 'string' ? (res as any).unbind_tip : '确认解绑当前设备吗？',
      notice: typeof (res as any).notice === 'string' ? (res as any).notice : null,
    }
  } catch {
    cachedAppConfig = { unbindTip: '确认解绑当前设备吗？', notice: null }
  }
  return cachedAppConfig
}

export function getUnbindTip(): string {
  return cachedAppConfig?.unbindTip || '确认解绑当前设备吗？'
}
