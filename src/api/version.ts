import axios from 'axios'

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

export interface UploadExeResult {
  url: string
  filename: string
  sha256?: string
}

/** 检查更新（客户端用，不需要登录、不需要签名） */
export async function checkUpdate(appId: string, currentVersion: string): Promise<CheckUpdateResult> {
  const res = await axios.post(`${BASE_URL}/client/version/check-update`, {
    app_id: appId,
    current_version: currentVersion,
  })
  return res.data?.data ?? res.data
}

/** 获取下一个版本号（不需要 token） */
export async function fetchNextVersion(appId: string): Promise<{ current: string; next: string }> {
  const res = await axios.get(`${BASE_URL}/huayun/api/versions/next-version`, {
    params: { app_id: appId },
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

/** 上传 EXE 安装包 */
export async function uploadExe(
  token: string,
  appId: string,
  version: string,
  file: File,
  onProgress?: (percent: number) => void,
): Promise<UploadExeResult> {
  const form = new FormData()
  form.append('app_id', appId)
  form.append('version', version)
  form.append('token', token)
  form.append('file', file)

  const res = await axios.post(`${BASE_URL}/huayun/api/versions/upload-exe`, form, {
    headers: { 'Content-Type': 'multipart/form-data' },
    onUploadProgress(e) {
      if (e.total && onProgress) onProgress(Math.round((e.loaded / e.total) * 100))
    },
  })
  return res.data?.data ?? res.data
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
