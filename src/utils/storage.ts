/**
 * 按 app_id 隔离的 localStorage 封装。
 * 所有 key 自动添加 "{appId}_" 前缀，不同应用的数据互不干扰。
 *
 * 使用前必须调用 initAppStorage() 完成初始化。
 */

let prefix = ''

export async function initAppStorage(): Promise<void> {
  const { getAppCredentials } = await import('./config')
  const { appId } = await getAppCredentials()
  prefix = `${appId}_`
}

function prefixedKey(key: string): string {
  return prefix + key
}

export const appStorage = {
  getItem(key: string): string | null {
    return localStorage.getItem(prefixedKey(key))
  },

  setItem(key: string, value: string): void {
    localStorage.setItem(prefixedKey(key), value)
  },

  removeItem(key: string): void {
    localStorage.removeItem(prefixedKey(key))
  },

  clear(): void {
    const keysToRemove: string[] = []
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i)
      if (k?.startsWith(prefix)) keysToRemove.push(k)
    }
    keysToRemove.forEach(k => localStorage.removeItem(k))
  },
}
