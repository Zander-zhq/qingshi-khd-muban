import { post } from '../utils/request'

export interface UserLoginParams {
  app_id: string
  acctno: string
  password: string
  device_id: string
  instance_id: string
  brand_id?: string
}

export interface UserRegisterParams {
  app_id: string
  phone: string
  password: string
  device_id: string
  instance_id?: string
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
  instance_id: string
}

export interface LogoutParams {
  app_id: string
  token: string
  device_id: string
  instance_id: string
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

/* ─── 签到 ─── */

export interface CheckinInfo {
  checked_today: boolean
  can_checkin: boolean
  reward_type: string
  reward_value: number
  reward_summary: string
  brand_id: string
}

export interface CheckinResult {
  message: string
  reward_applied: boolean
  reward_type: string
  reward_value: number
  reward_summary: string
  vip_expire_at?: string
  fen?: number
  checkin: CheckinInfo
}

export function userCheckinApi(data: { app_id: string; token: string; brand_id?: string }) {
  return post<CheckinResult>('/client/user/checkin', data)
}

