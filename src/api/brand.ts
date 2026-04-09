import { post } from '../utils/request'
import { fetch } from '@tauri-apps/plugin-http'
import { createSignHeaders } from '../utils/sign'
import { getAppCredentials } from '../utils/config'

/* ─── Template ID ↔ 服务端 label 映射 ─── */

const TEMPLATE_TO_LABEL: Record<string, string> = {
  green: '经典（绿色）',
  orange: '活力（橙色）',
  dark: '科技（暗黑）',
}
const LABEL_TO_TEMPLATE: Record<string, string> = Object.fromEntries(
  Object.entries(TEMPLATE_TO_LABEL).map(([k, v]) => [v, k]),
)

export function templateIdToLabel(id: string): string {
  return TEMPLATE_TO_LABEL[id] || id
}

export function templateLabelToId(label: string): string {
  return LABEL_TO_TEMPLATE[label] || label
}

/* ─── 服务端品牌结构 ─── */

/** 与后台「编辑品牌」奖励控制一致；服务端可能为 member/points 或数字枚举 */
export type ServerRewardType = string | number

export interface ServerBrand {
  id: number
  brand_id: string
  name: string
  product_name: string
  template: string
  about_info: string
  logo: string
  website_url: string
  tutorial_url: string
  contact_images: string[]
  data_version: number
  created_at: string
  updated_at: string
  reg_reward_type?: ServerRewardType
  reg_reward_value?: number
  invite_invitee_reward_type?: ServerRewardType
  invite_invitee_reward_value?: number
  invite_inviter_reward_type?: ServerRewardType
  invite_inviter_reward_value?: number
  checkin_reward_type?: ServerRewardType
  checkin_reward_value?: number
  pay_channel?: string
  pay_methods?: string[]
  disclaimer?: string
}

/* ─── 品牌 CRUD ─── */

export async function fetchBrandList(token: string): Promise<{ total: number; items: ServerBrand[] }> {
  const { appId } = await getAppCredentials()
  return post<{ total: number; items: ServerBrand[] }>(`/client/brands/list?app_id=${appId}`, { token })
}

export async function fetchBrandDetail(token: string, brandId: string): Promise<ServerBrand> {
  return post<ServerBrand>(`/client/brands/detail?brand_id=${brandId}`, { token })
}

export async function createBrand(token: string, data: {
  name: string
  product_name: string
  template: string
  about_info: string
  logo?: string
  website_url?: string
  tutorial_url?: string
  contact_images?: string[]
  reg_reward_type?: ServerRewardType
  reg_reward_value?: number
  invite_invitee_reward_type?: ServerRewardType
  invite_invitee_reward_value?: number
  invite_inviter_reward_type?: ServerRewardType
  invite_inviter_reward_value?: number
  checkin_reward_type?: ServerRewardType
  checkin_reward_value?: number
  pay_channel?: string
  pay_methods?: string[]
  disclaimer?: string
}): Promise<{ id: number; brand_id: string }> {
  const { appId } = await getAppCredentials()
  return post<{ id: number; brand_id: string }>('/client/brands/create', {
    token,
    app_id: appId,
    ...data,
  })
}

export async function updateBrand(
  token: string,
  brandId: string,
  data: Record<string, unknown>,
): Promise<ServerBrand> {
  return post<ServerBrand>(`/client/brands/update?brand_id=${brandId}`, { token, ...data })
}

export async function deleteBrandApi(token: string, brandId: string): Promise<void> {
  await post<void>(`/client/brands/delete?brand_id=${brandId}`, { token })
}

/* ─── 品牌加密配置同步 ─── */

export interface SyncConfigResult {
  updated: boolean
  data_version: number
  brand_id?: string
  config?: string
}

export async function syncBrandConfig(
  brandId?: string,
  dataVersion: number = 0,
): Promise<SyncConfigResult> {
  const { appId } = await getAppCredentials()
  const body: Record<string, unknown> = { data_version: dataVersion }
  if (brandId) body.brand_id = brandId
  return post<SyncConfigResult>(`/client/brands/sync-config?app_id=${appId}`, body)
}

/* ─── 统一图片上传 ─── */

export async function uploadImage(
  token: string,
  file: File,
  type: 'logo' | 'avatar' | 'contact',
  options: { brandId?: string; phone?: string } = {},
): Promise<{ url: string }> {
  const { appId } = await getAppCredentials()
  const BASE_URL = import.meta.env.VITE_API_BASE_URL as string

  const params = new URLSearchParams({ app_id: appId, type })
  if (options.brandId) params.set('brand_id', options.brandId)
  if (options.phone) params.set('phone', options.phone)

  const signHeaders = await createSignHeaders({ token })

  const formData = new FormData()
  formData.append('token', token)
  formData.append('file', file)

  const resp = await fetch(`${BASE_URL}/client/upload/image?${params.toString()}`, {
    method: 'POST',
    headers: { ...signHeaders },
    body: formData,
  })

  const text = await resp.text()
  if (!resp.ok) throw new Error(`服务器错误 (${resp.status})`)

  let res: { code: number; msg: string; data: { url: string } }
  try {
    res = JSON.parse(text)
  } catch {
    throw new Error('服务器返回了无效的响应')
  }

  if (res.code !== 0) throw new Error(res.msg || '上传失败')
  return res.data
}
