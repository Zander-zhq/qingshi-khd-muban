import { post } from '../utils/request'

export interface PayPackage {
  id: number
  name: string
  card_type: string
  price: number
  duration_seconds?: number
  points?: number
  description?: string
}

export interface CreateOrderResult {
  order_no: string
  code_url?: string
  qr_code?: string
  pay_url?: string
}

export interface OrderStatus {
  order_no: string
  status: 'pending' | 'paid' | 'failed'
  paid_at?: string
}

export function fetchPackages(token: string, appId: string) {
  return post<{ items: PayPackage[] }>(`/client/pay/packages?app_id=${appId}`, { token })
}

export function createOrder(token: string, appId: string, data: {
  card_group_id: number
  payment_method: string
}) {
  return post<CreateOrderResult>(`/client/pay/create-order?app_id=${appId}`, { token, ...data })
}

export function queryOrder(token: string, orderNo: string) {
  return post<OrderStatus>('/client/pay/query-order', { token, order_no: orderNo })
}
