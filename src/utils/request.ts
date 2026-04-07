import axios from 'axios'
import type { AxiosInstance, AxiosRequestConfig, AxiosResponse, InternalAxiosRequestConfig } from 'axios'
import { logger } from './logger'
import { createSignHeaders } from './sign'
import { appStorage } from './storage'

const BASE_URL = import.meta.env.VITE_API_BASE_URL as string

/** 所有 /client/* 接口（除 /client/init）都需要签名 */
function needsSign(url: string | undefined): boolean {
  if (!url) return false
  if (!url.includes('/client/')) return false
  if (url.includes('/client/init')) return false
  return true
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
    const token = appStorage.getItem('token')
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }

    const isFormData = config.data instanceof FormData ||
      (config.data && typeof config.data === 'object' && typeof config.data.append === 'function')

    if (isFormData) {
      config.headers.delete('Content-Type')
    }

    if (needsSign(config.url) && config.data) {
      let body: Record<string, unknown>
      if (isFormData) {
        body = {}
        ;(config.data as FormData).forEach((value: FormDataEntryValue, key: string) => {
          if (typeof value === 'string') body[key] = value
        })
      } else if (typeof config.data === 'string') {
        body = JSON.parse(config.data)
      } else {
        body = JSON.parse(JSON.stringify(config.data))
      }
      const signHeaders = await createSignHeaders(body)
      config.headers['X-App-Id'] = signHeaders['X-App-Id']
      config.headers['X-Timestamp'] = signHeaders['X-Timestamp']
      config.headers['X-Nonce'] = signHeaders['X-Nonce']
      config.headers['X-Sign'] = signHeaders['X-Sign']
    }

    logger.log('request', '请求发出', {
      method: config.method,
      url: config.url,
      hasToken: !!token,
      data: config.data,
      signHeaders: needsSign(config.url) ? {
        'X-App-Id': config.headers['X-App-Id'],
        'X-Timestamp': config.headers['X-Timestamp'],
        'X-Nonce': config.headers['X-Nonce'],
        'X-Sign': config.headers['X-Sign'],
      } : undefined,
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
      const err = new Error(res.msg || '请求失败') as Error & { code: number }
      err.code = res.code
      return Promise.reject(err)
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
    const detail = error.response?.data?.detail
    const message = error.response?.data?.msg
      || (Array.isArray(detail) ? detail.map((d: any) => d.msg || JSON.stringify(d)).join('; ') : (typeof detail === 'string' ? detail : null))
      || error.message || '请求失败'
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
