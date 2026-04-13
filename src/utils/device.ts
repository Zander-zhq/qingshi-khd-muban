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

const INSTANCE_ID_KEY = '__qingshi_instance_id__'

function initInstanceId(): string {
  const stored = sessionStorage.getItem(INSTANCE_ID_KEY)
  if (stored) return stored
  const id = crypto.randomUUID()
  sessionStorage.setItem(INSTANCE_ID_KEY, id)
  return id
}

const instanceId = initInstanceId()

export function getInstanceId(): string {
  return instanceId
}
