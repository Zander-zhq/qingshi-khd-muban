import { invoke } from '@tauri-apps/api/core'
import { getDeviceId, getInstanceId } from './device'

const API_BASE = import.meta.env.VITE_API_BASE_URL as string

export async function registerSessionToRust(token: string): Promise<void> {
  try {
    const deviceId = await getDeviceId()
    await invoke('register_session', {
      token,
      deviceId,
      instanceId: getInstanceId(),
      apiBaseUrl: API_BASE,
    })
  } catch { /* non-critical */ }
}

export async function clearSessionFromRust(): Promise<void> {
  try {
    await invoke('clear_session')
  } catch { /* non-critical */ }
}
