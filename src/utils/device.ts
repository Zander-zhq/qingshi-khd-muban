import { invoke } from '@tauri-apps/api/core'

let cachedDeviceId: string | null = null

export async function getDeviceId(): Promise<string> {
  if (cachedDeviceId) return cachedDeviceId
  try {
    cachedDeviceId = await invoke<string>('get_device_id')
  } catch {
    cachedDeviceId = 'UNKNOWN-' + Date.now().toString(36)
  }
  return cachedDeviceId
}

const instanceId = crypto.randomUUID()

export function getInstanceId(): string {
  return instanceId
}
