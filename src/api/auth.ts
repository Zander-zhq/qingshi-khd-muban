import { post } from '../utils/request'

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
  sms_code?: string
  nickname?: string
  invite_code?: string
  email?: string
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

export function userLoginApi(data: UserLoginParams) {
  return post<Record<string, unknown>>('/client/user/login', data)
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
