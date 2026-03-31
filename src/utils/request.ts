import axios from 'axios'
import type { AxiosInstance, AxiosRequestConfig, AxiosResponse, InternalAxiosRequestConfig } from 'axios'
import { invoke } from '@tauri-apps/api/core'
import { logger } from './logger'

const BASE_URL = import.meta.env.VITE_API_BASE_URL as string

const SIGN_REQUIRED_PREFIXES = [
  '/client/user/',
  '/client/login',
  '/client/heartbeat',
  '/client/logout',
]

interface SignResult {
  app_id: string
  timestamp: string
  nonce: string
  sign: string
}

function needsSign(url: string | undefined): boolean {
  if (!url) return false
  return SIGN_REQUIRED_PREFIXES.some((p) => url.includes(p))
}

const service: AxiosInstance = axios.create({
  baseURL: BASE_URL,
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

service.interceptors.request.use(
  async (config: InternalAxiosRequestConfig) => {
    const token = localStorage.getItem('token')
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }

    if (needsSign(config.url) && config.data) {
      const bodyJson = typeof config.data === 'string' ? config.data : JSON.stringify(config.data)
      const signResult = await invoke<SignResult>('compute_sign', { bodyJson })
      config.headers['X-App-Id'] = signResult.app_id
      config.headers['X-Timestamp'] = signResult.timestamp
      config.headers['X-Nonce'] = signResult.nonce
      config.headers['X-Sign'] = signResult.sign
      logger.log('request', '签名完成', {
        method: config.method,
        url: config.url,
        hasToken: !!token,
        headers: {
          'X-App-Id': signResult.app_id,
          'X-Timestamp': signResult.timestamp,
          'X-Nonce': signResult.nonce,
          'X-Sign': signResult.sign,
        },
      })
    }

    logger.log('request', '请求发出前', {
      baseURL: config.baseURL,
      method: config.method,
      url: config.url,
      hasToken: !!token,
      params: config.params,
      data: config.data,
    })

    return config
  },
  (error) => {
    logger.error('request', '请求拦截器异常', {
      message: error?.message,
      stack: error?.stack,
    })
    return Promise.reject(error)
  },
)

service.interceptors.response.use(
  (response: AxiosResponse) => {
    const res = response.data
    logger.log('response', '收到响应', {
      method: response.config.method,
      url: response.config.url,
      status: response.status,
      data: res,
    })
    if (res.code !== undefined && res.code !== 0) {
      logger.warn('response', '业务响应非成功', {
        method: response.config.method,
        url: response.config.url,
        status: response.status,
        code: res.code,
        msg: res.msg,
      })
      return Promise.reject(new Error(res.msg || '请求失败'))
    }

    if (res.code !== undefined) {
      const normalized = res.data ?? {}
      logger.log('response', '响应已解包为 data', {
        method: response.config.method,
        url: response.config.url,
        keys: typeof normalized === 'object' && normalized ? Object.keys(normalized) : [],
      })
      return normalized
    }

    return res
  },
  (error) => {
    logger.error('response', '请求失败', {
      method: error.config?.method,
      url: error.config?.url,
      status: error.response?.status,
      statusText: error.response?.statusText,
      responseData: error.response?.data,
      message: error.message,
    })
    const message = error.response?.data?.msg || error.response?.data?.detail || error.message || '请求失败'
    return Promise.reject(new Error(message))
  },
)

export function get<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  return service.get(url, config)
}

export function post<T>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
  return service.post(url, data, config)
}

export function put<T>(url: string, data?: unknown, config?: AxiosRequestConfig): Promise<T> {
  return service.put(url, data, config)
}

export function del<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  return service.delete(url, config)
}

export default service
