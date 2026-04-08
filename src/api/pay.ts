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

/* ─── Guest（不登录）版在线支付 ─── */

export interface GuestCreateOrderData {
  acctno: string
  card_group_id: number
  payment_method: string
  brand_id?: string
}

export interface GuestOrderStatus extends OrderStatus {
  amount?: number
  payment_method?: string
  card_group_name?: string
  vip_expire_at?: string
  fen?: number
}

export function fetchGuestPackages(appId: string) {
  return post<{ items: PayPackage[] }>(`/client/pay/guest/packages?app_id=${appId}`, {})
}

export function createGuestOrder(appId: string, data: GuestCreateOrderData) {
  return post<CreateOrderResult>(`/client/pay/guest/create-order?app_id=${appId}`, data)
}

export function queryGuestOrder(orderNo: string) {
  return post<GuestOrderStatus>('/client/pay/guest/query-order', { order_no: orderNo })
}
