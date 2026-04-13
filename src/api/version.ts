import axios from 'axios'
import { post } from '../utils/request'

const BASE_URL = import.meta.env.VITE_API_BASE_URL as string

export interface UpdateVersion {
  id?: number
  version: string
  description: string | null
  force_update: boolean
  download_url?: string
  created_at: string
}

export interface CheckUpdateResult {
  has_update: boolean
  current_version: string
  latest_version: string
  download_url: string | null
  file_hash: string | null
  force_update: boolean
  updates: UpdateVersion[]
}

export interface UploadTokenResult {
  upload_url: string
  download_url: string
  cos_key: string
  content_type: string
}

export interface UploadExeResult {
  url: string
  download_url: string
}

/** 检查更新（客户端用，不需要登录、不需要签名） */
export async function checkUpdate(appId: string, currentVersion: string): Promise<CheckUpdateResult> {
  const res = await axios.post(`${BASE_URL}/client/version/check-update`, {
    app_id: appId,
    current_version: currentVersion,
  })
  return res.data?.data ?? res.data
}

/** 获取下一个版本号 */
export async function fetchNextVersion(appId: string, token?: string): Promise<{ current: string; next: string }> {
  const res = await axios.get(`${BASE_URL}/huayun/api/versions/next-version`, {
    params: { app_id: appId },
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  })
  return res.data?.data ?? res.data
}

/** 创建版本记录 */
export async function createVersion(
  token: string,
  data: {
    app_id: string
    version: string
    description?: string
    force_update?: boolean
  },
): Promise<{ id: number }> {
  const res = await axios.post(`${BASE_URL}/huayun/api/versions`, {
    token,
    ...data,
  })
  return res.data?.data ?? res.data
}

/** 1. 获取上传凭证（预签名 URL） */
export function getUploadToken(data: {
  token: string
  app_id: string
  filename: string
  type: 'exe' | 'logo' | 'avatar' | 'contact'
  version?: string
  brand_id?: string | null
  phone?: string | null
}) {
  return post<UploadTokenResult>('/client/upload/token', data)
}

/** 2. PUT 直传 COS（使用 Tauri HTTP 插件绕过浏览器 CORS） */
async function putToCos(
  uploadUrl: string,
  fileData: Uint8Array,
  contentType: string,
  onProgress?: (percent: number) => void,
): Promise<void> {
  const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
  const resp = await tauriFetch(uploadUrl, {
    method: 'PUT',
    headers: { 'Content-Type': contentType },
    body: fileData,
  })
  if (!resp.ok) {
    const text = await resp.text().catch(() => '')
    throw new Error(`COS 上传失败: HTTP ${resp.status} ${text.slice(0, 200)}`)
  }
  if (onProgress) onProgress(100)
}

/** 3. 安装包上传确认 */
export function confirmExeUpload(data: {
  token: string
  app_id: string
  version: string
  download_url: string
  description?: string
  force_update?: boolean
}) {
  return post<Record<string, unknown>>('/client/version/upload-confirm', data)
}

/** 上传 EXE 安装包（获取凭证 → 直传 COS → 确认） */
export async function uploadExe(
  token: string,
  appId: string,
  version: string,
  fileData: Uint8Array,
  fileName: string,
  onProgress?: (percent: number) => void,
  onLog?: (msg: string) => void,
): Promise<UploadExeResult> {
  const log = onLog || (() => {})

  log('正在获取上传凭证…')
  const credential = await getUploadToken({
    token, app_id: appId, filename: fileName, type: 'exe', version,
  })
  log(`凭证获取成功，COS Key: ${credential.cos_key}`)

  log('正在直传 COS…')
  await putToCos(credential.upload_url, fileData, credential.content_type, onProgress)
  log(`COS 上传完成，下载地址: ${credential.download_url}`)

  return { url: credential.download_url, download_url: credential.download_url }
}

/** 编辑版本（更新下载地址等） */
export async function updateVersion(
  token: string,
  versionId: number,
  data: {
    download_url?: string
    description?: string
    force_update?: boolean
  },
): Promise<void> {
  await axios.put(`${BASE_URL}/huayun/api/versions/${versionId}`, {
    token,
    ...data,
  })
}

/** 删除版本 */
export async function deleteVersion(token: string, versionId: number): Promise<void> {
  await axios.delete(`${BASE_URL}/huayun/api/versions/${versionId}`, {
    params: { token },
  })
}
