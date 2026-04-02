import { post } from '../utils/request'
import { fetch } from '@tauri-apps/plugin-http'
import { createSignHeaders } from '../utils/sign'

export interface UserLoginParams {
  app_id: string
  acctno: string
  password: string
  device_id: string
}

export interface UserRegisterParams {
  app_id: string
  phone: string
  password: string
  device_id: string
  acctno?: string
  nickname?: string
  invite_code?: string
}

export interface InitParams {
  app_id: string
  app_key: string
}

export interface HeartbeatParams {
  app_id: string
  token: string
  device_id: string
}

export interface LogoutParams {
  app_id: string
  token: string
  device_id: string
}

export function userLoginApi(data: UserLoginParams, config?: { signal?: AbortSignal }) {
  return post<Record<string, unknown>>('/client/user/login', data, config)
}

export function userRegisterApi(data: UserRegisterParams) {
  return post<Record<string, unknown>>('/client/user/register', data)
}

// TODO: 待后端短信接口就绪后实现
export function sendSmsCodeApi(data: { phone: string }) {
  return post<Record<string, unknown>>('/client/sms/send', data)
}

export function clientInitApi(data: InitParams) {
  return post<Record<string, unknown>>('/client/init', data)
}

export function userHeartbeatApi(data: HeartbeatParams) {
  return post<Record<string, unknown>>('/client/user/heartbeat', data)
}

export function userLogoutApi(data: LogoutParams) {
  return post<Record<string, unknown>>('/client/user/logout', data)
}

export interface UpdateProfileParams {
  token: string
  nickname?: string
  acctno?: string
  email?: string
  avatars?: string
}

export function updateProfileApi(data: UpdateProfileParams) {
  return post<Record<string, unknown>>('/client/user/profile', data)
}

export function sendEmailCodeApi(data: { app_id: string; email: string; scene: 'bind_email' | 'reset_password' }) {
  return post<Record<string, unknown>>('/client/user/send-email-code', data)
}

export function bindEmailApi(data: { token: string; email: string; code: string }) {
  return post<Record<string, unknown>>('/client/user/bind-email', data)
}

export function resetPasswordApi(data: { app_id: string; email: string; code: string; new_password: string }) {
  return post<Record<string, unknown>>('/client/user/reset-password', data)
}

export function changePasswordApi(data: { token: string; old_password: string; new_password: string }) {
  return post<Record<string, unknown>>('/client/user/change-password', data)
}

export function unbindDeviceApi(data: { app_id: string; acctno: string; password: string; device_sn?: string }) {
  return post<Record<string, unknown>>('/client/user/unbind-device', data)
}

export function unbindDeviceInnerApi(data: { token: string; device_id: string }) {
  return post<Record<string, unknown>>('/client/user/unbind-device-inner', data)
}

export function redeemCardApi(data: { app_id: string; acctno: string; card_key: string }) {
  return post<Record<string, unknown>>('/client/user/redeem-card', data)
}

export function redeemCardInnerApi(data: { token: string; card_key: string }) {
  return post<Record<string, unknown>>('/client/user/redeem-card-inner', data)
}

export async function uploadAvatarApi(token: string, file: File): Promise<Record<string, unknown>> {
  const BASE_URL = import.meta.env.VITE_API_BASE_URL as string
  const signHeaders = await createSignHeaders({ token })

  const formData = new FormData()
  formData.append('token', token)
  formData.append('file', file)

  const resp = await fetch(`${BASE_URL}/client/user/avatar`, {
    method: 'POST',
    headers: {
      ...signHeaders,
    },
    body: formData,
  })

  const text = await resp.text()
  if (!resp.ok) {
    throw new Error(`服务器错误 (${resp.status})，请稍后重试`)
  }

  let res: { code: number; msg: string; data: Record<string, unknown> }
  try {
    res = JSON.parse(text)
  } catch {
    throw new Error('服务器返回了无效的响应')
  }

  if (res.code !== 0) {
    throw new Error(res.msg || '上传失败')
  }
  return res.data ?? {}
}
