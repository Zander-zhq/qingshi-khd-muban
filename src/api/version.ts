import axios from 'axios'

const BASE_URL = import.meta.env.VITE_API_BASE_URL as string

export interface UpdateVersion {
  version: string
  description: string | null
  force_update: boolean
  created_at: string
}

export interface CheckUpdateResult {
  has_update: boolean
  current_version: string
  latest_version: string
  download_url: string | null
  force_update: boolean
  updates: UpdateVersion[]
}

export interface UploadExeResult {
  url: string
  filename: string
}

/** 检查更新（不需要登录、不需要签名） */
export async function checkUpdate(appId: string, currentVersion: string): Promise<CheckUpdateResult> {
  const res = await axios.post(`${BASE_URL}/client/version/check-update`, {
    app_id: appId,
    current_version: currentVersion,
  })
  return res.data?.data ?? res.data
}

/** 上传 EXE 安装包（管理员接口） */
export async function uploadExe(
  adminToken: string,
  appId: string,
  version: string,
  file: File,
  onProgress?: (percent: number) => void,
): Promise<UploadExeResult> {
  const form = new FormData()
  form.append('app_id', appId)
  form.append('version', version)
  form.append('file', file)

  const res = await axios.post(`${BASE_URL}/huayun/api/versions/upload-exe`, form, {
    headers: {
      Authorization: `Bearer ${adminToken}`,
      'Content-Type': 'multipart/form-data',
    },
    onUploadProgress(e) {
      if (e.total && onProgress) onProgress(Math.round((e.loaded / e.total) * 100))
    },
  })
  return res.data?.data ?? res.data
}

/** 创建/发布版本（管理员接口） */
export async function publishVersion(
  adminToken: string,
  data: {
    app_id: string
    version: string
    description?: string
    force_update?: boolean
    download_url?: string
  },
): Promise<void> {
  await axios.post(`${BASE_URL}/huayun/api/versions/create`, data, {
    headers: { Authorization: `Bearer ${adminToken}` },
  })
}

/** 获取版本列表（管理员接口） */
export async function fetchVersions(adminToken: string, appId: string): Promise<UpdateVersion[]> {
  const res = await axios.get(`${BASE_URL}/huayun/api/versions/list`, {
    headers: { Authorization: `Bearer ${adminToken}` },
    params: { app_id: appId },
  })
  return res.data?.data?.items ?? res.data?.data ?? []
}
