import { userHeartbeatApi, userLogoutApi } from '../api/auth'
import { getAppCredentials } from './config'
import { getDeviceId } from './device'
import { logger } from './logger'

const HEARTBEAT_INTERVAL = 60_000
const MAX_CONSECUTIVE_FAILURES = 3

let timer: ReturnType<typeof setInterval> | null = null
let consecutiveFailures = 0

export interface HeartbeatCallbacks {
  onBanned?: (msg: string) => void
  onExpired?: (msg: string) => void
  onSessionExpired?: (msg: string) => void
  onDeviceMismatch?: (msg: string) => void
  onDeviceKicked?: (msg: string) => void
  onServerUnreachable?: (msg: string) => void
}

let callbacks: HeartbeatCallbacks = {}

export function setHeartbeatCallbacks(cbs: HeartbeatCallbacks) {
  callbacks = cbs
}

async function sendHeartbeat(token: string) {
  try {
    const [{ appId }, deviceId] = await Promise.all([getAppCredentials(), getDeviceId()])
    await userHeartbeatApi({ app_id: appId, token, device_id: deviceId })
    consecutiveFailures = 0
    logger.log('heartbeat', '心跳发送成功')
  } catch (err) {
    const code = (err as any)?.code as number | undefined
    const msg = err instanceof Error ? err.message : String(err)
    logger.warn('heartbeat', '心跳异常', { code, message: msg })

    if (code === -3) {
      stopHeartbeat()
      callbacks.onBanned?.(msg)
    } else if (code === -4) {
      stopHeartbeat()
      callbacks.onExpired?.(msg)
    } else if (code === -2) {
      stopHeartbeat()
      callbacks.onSessionExpired?.(msg)
    } else if (code === -5) {
      stopHeartbeat()
      callbacks.onDeviceKicked?.(msg)
    } else if (code === -1 && msg.includes('设备')) {
      stopHeartbeat()
      callbacks.onDeviceMismatch?.(msg)
    } else {
      consecutiveFailures++
      logger.warn('heartbeat', `连续失败 ${consecutiveFailures}/${MAX_CONSECUTIVE_FAILURES}`, { code, message: msg })
      if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
        stopHeartbeat()
        callbacks.onServerUnreachable?.(
          `无法连接到服务器（已连续 ${consecutiveFailures} 次失败），请检查网络后重新登录`
        )
      }
    }
  }
}

export function startHeartbeat(token: string) {
  stopHeartbeat()
  consecutiveFailures = 0
  logger.log('heartbeat', '启动心跳', { interval: HEARTBEAT_INTERVAL })
  sendHeartbeat(token)
  timer = setInterval(() => sendHeartbeat(token), HEARTBEAT_INTERVAL)
}

export function stopHeartbeat() {
  if (timer) {
    clearInterval(timer)
    timer = null
    logger.log('heartbeat', '心跳已停止')
  }
}

export async function callLogoutApi(token: string) {
  try {
    const [{ appId }, deviceId] = await Promise.all([getAppCredentials(), getDeviceId()])
    await userLogoutApi({ app_id: appId, token, device_id: deviceId })
    logger.log('logout', '退出接口调用成功')
  } catch (err) {
    logger.warn('logout', '退出接口调用失败', {
      message: err instanceof Error ? err.message : String(err),
    })
  }
}
