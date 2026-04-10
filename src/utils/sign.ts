import CryptoJS from 'crypto-js'
import { getAppCredentials } from './config'
import { logger } from './logger'

function generateNonce(length: number = 16): string {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let result = ''
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  return result
}

/**
 * 时间偏差补偿：本地时间 + offsetSeconds = 服务器时间
 * 启动时调用 syncServerTime() 校准一次
 */
let timeOffsetSeconds = 0
let timeSynced = false

export async function syncServerTime() {
  try {
    const BASE_URL = import.meta.env.VITE_API_BASE_URL as string
    const localBefore = Math.floor(Date.now() / 1000)
    const resp = await fetch(`${BASE_URL}/health`, { method: 'GET' })
    const localAfter = Math.floor(Date.now() / 1000)
    const dateHeader = resp.headers.get('Date')
    if (dateHeader) {
      const serverTime = Math.floor(new Date(dateHeader).getTime() / 1000)
      const localMid = Math.floor((localBefore + localAfter) / 2)
      timeOffsetSeconds = serverTime - localMid
      timeSynced = true
      logger.log('sign', '时间同步完成', {
        serverTime,
        localTime: localMid,
        offsetSeconds: timeOffsetSeconds,
      })
    }
  } catch {
    logger.warn('sign', '时间同步失败，使用本地时间')
  }
}

function getCorrectedTimestamp(): string {
  return Math.floor(Date.now() / 1000 + timeOffsetSeconds).toString()
}

export function isTimeSynced(): boolean {
  return timeSynced
}

export function getTimeOffset(): number {
  return timeOffsetSeconds
}

function buildSignString(body: Record<string, unknown>, timestamp: string, nonce: string): string {
  const filtered: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(body)) {
    if (value !== null && value !== undefined && key !== 'sign') {
      filtered[key] = value
    }
  }

  const sortedKeys = Object.keys(filtered).sort()
  const parts = sortedKeys.map(key => {
    const v = filtered[key]
    return `${key}=${Array.isArray(v) || (typeof v === 'object' && v !== null) ? JSON.stringify(v) : v}`
  })
  parts.push(`timestamp=${timestamp}`)
  parts.push(`nonce=${nonce}`)
  return parts.join('&')
}

function computeHmacSign(signString: string, appKey: string): string {
  return CryptoJS.HmacSHA256(signString, appKey).toString(CryptoJS.enc.Hex)
}

export interface SignHeaders {
  'X-App-Id': string
  'X-Timestamp': string
  'X-Nonce': string
  'X-Sign': string
}

export async function createSignHeaders(body: Record<string, unknown>): Promise<SignHeaders> {
  const { appId, appKey } = await getAppCredentials()
  const timestamp = getCorrectedTimestamp()
  const nonce = generateNonce()
  const signString = buildSignString(body, timestamp, nonce)
  const sign = computeHmacSign(signString, appKey)

  if (import.meta.env.DEV) {
    logger.log('sign', '签名计算', {
      bodyKeys: Object.keys(body),
      signString,
      sign,
    })
  }

  return {
    'X-App-Id': appId,
    'X-Timestamp': timestamp,
    'X-Nonce': nonce,
    'X-Sign': sign,
  }
}
